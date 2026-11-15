//! UI Automation reading of the destination (F20, F21, F23, F24, F26, F27,
//! F32, F34). Every UIA call runs on one dedicated worker thread with a
//! deadline; a worker that stops answering is abandoned and replaced.

use super::{
    classify, is_browser, is_excluded, is_placeholder, names_near_emails, normalize_host,
    normalize_newlines, process_key, Capture, Field,
};
use std::collections::VecDeque;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use windows::core::{Interface, BOOL, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
};

/// Whole reading budget (F34). Past it, the dictation goes on without context.
const DEADLINE: Duration = Duration::from_millis(700);
/// Page text walk budget, inside the deadline.
const PAGE_BUDGET: Duration = Duration::from_millis(300);
const PAGE_NODES: usize = 600;
const PAGE_CHARS: usize = 12_000;
/// Field text read around the caret (trimmed to the sent size later).
const FIELD_CHARS: i32 = 1_000;

/// Localized names of the browsers' address bar (PAR §3.2).
const ADDRESS_BAR_NAMES: &[&str] = &[
    "Address and search bar",
    "Barre d'adresse et de recherche",
    "Adress- und Suchleiste",
    "Barra de direcciones y de búsqueda",
    "Barra degli indirizzi e di ricerca",
    "Address bar",
    "Search or enter address",
    "Search with Google or enter address",
    "Saisir une recherche ou une adresse",
    "Rechercher avec Google ou saisir une adresse",
];

type Job = Box<dyn FnOnce(&IUIAutomation) + Send>;

static WORKER: Mutex<Option<mpsc::Sender<Job>>> = Mutex::new(None);

fn spawn_worker() -> Option<mpsc::Sender<Job>> {
    let (tx, rx) = mpsc::channel::<Job>();
    std::thread::Builder::new()
        .name("uia-context".into())
        .spawn(move || {
            // SAFETY: COM is initialized on this thread before any UIA call.
            let automation: IUIAutomation = unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
                    Ok(a) => a,
                    Err(e) => {
                        log::warn!("UI Automation is unavailable: {e}");
                        return;
                    }
                }
            };
            if let Ok(a2) = automation.cast::<IUIAutomation2>() {
                // SAFETY: plain setters on a live interface.
                unsafe {
                    let _ = a2.SetConnectionTimeout(1_000);
                    let _ = a2.SetTransactionTimeout(500);
                }
            }
            for job in rx {
                job(&automation);
            }
        })
        .ok()?;
    Some(tx)
}

fn reset_worker() {
    *WORKER.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

/// Runs `f` on the UIA worker; `None` if it doesn't answer in time.
fn run<T: Send + 'static>(
    timeout: Duration,
    f: impl FnOnce(&IUIAutomation) -> T + Send + 'static,
) -> Option<T> {
    let (reply_tx, reply_rx) = mpsc::channel();
    let job: Job = Box::new(move |a| {
        let _ = reply_tx.send(f(a));
    });
    {
        let mut worker = WORKER.lock().unwrap_or_else(|e| e.into_inner());
        if worker.is_none() {
            *worker = spawn_worker();
        }
        let sent = worker.as_ref().is_some_and(|tx| tx.send(job).is_ok());
        if !sent {
            *worker = None;
            return None;
        }
    }
    match reply_rx.recv_timeout(timeout) {
        Ok(v) => Some(v),
        Err(RecvTimeoutError::Timeout) => {
            log::warn!("The focused app didn't answer UI Automation in {timeout:?}; dictating without context");
            // The stuck thread is left to finish on its own.
            reset_worker();
            None
        }
        Err(RecvTimeoutError::Disconnected) => {
            reset_worker();
            None
        }
    }
}

fn process_name(pid: u32) -> String {
    // SAFETY: the handle is checked and closed; the buffer outlives the call.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return String::new();
        };
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut len,
        );
        let _ = CloseHandle(process);
        if ok.is_err() {
            return String::new();
        }
        process_key(&String::from_utf16_lossy(&buf[..len as usize]))
    }
}

fn window_pid(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    // SAFETY: plain query with a valid out pointer.
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

/// Packaged apps (WhatsApp, Calculator…) run inside ApplicationFrameHost:
/// the real process owns a child window.
fn packaged_child_pid(hwnd: HWND, host_pid: u32) -> Option<u32> {
    unsafe extern "system" fn visit(child: HWND, data: LPARAM) -> BOOL {
        // SAFETY: `data` points at the (host pid, found pid) pair below.
        let pair = unsafe { &mut *(data.0 as *mut (u32, u32)) };
        let pid = window_pid(child);
        if pid != 0 && pid != pair.0 {
            pair.1 = pid;
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut pair = (host_pid, 0u32);
    // SAFETY: the callback only runs during this call, while `pair` lives.
    unsafe {
        let _ = EnumChildWindows(
            Some(hwnd),
            Some(visit),
            LPARAM(&mut pair as *mut _ as isize),
        );
    }
    (pair.1 != 0).then_some(pair.1)
}

fn bstr(r: windows::core::Result<windows::core::BSTR>) -> String {
    r.map(|b| b.to_string()).unwrap_or_default()
}

pub fn capture(read_context: bool, apps: &[String], domains: &[String]) -> Capture {
    let started = Instant::now();
    // SAFETY: plain Win32 queries on the foreground window.
    let hwnd = unsafe { GetForegroundWindow() };
    let mut cap = Capture::default();
    let mut owner = 0u32;
    if !hwnd.is_invalid() {
        owner = window_pid(hwnd);
        cap.process = process_name(owner);
        if cap.process == "applicationframehost" {
            if let Some(child) = packaged_child_pid(hwnd, owner) {
                owner = child;
                cap.process = process_name(child);
            }
        }
        let mut title = [0u16; 512];
        // SAFETY: the buffer outlives the call.
        let n = unsafe { GetWindowTextW(hwnd, &mut title) };
        cap.title = String::from_utf16_lossy(&title[..n.max(0) as usize]);
    }
    let (content, terminal) = classify(&cap.process, None);
    cap.content = Some(content);
    cap.terminal = terminal;
    cap.excluded = is_excluded(&cap.process, None, apps, domains);

    let base = cap.clone();
    let hwnd_raw = hwnd.0 as isize;
    let (apps, domains) = (apps.to_vec(), domains.to_vec());
    let cap = run(DEADLINE, move |a| {
        read(a, hwnd_raw, owner, cap, read_context, &apps, &domains)
    })
    .unwrap_or(base);
    log::debug!(
        "Context read in {:?}: {} {:?} {:?} sensitive={} excluded={} field={} names={}",
        started.elapsed(),
        cap.process,
        cap.host,
        cap.content,
        cap.sensitive,
        cap.excluded,
        cap.field.is_some(),
        cap.names.len()
    );
    cap
}

fn read(
    a: &IUIAutomation,
    hwnd: isize,
    owner: u32,
    mut cap: Capture,
    read_context: bool,
    apps: &[String],
    domains: &[String],
) -> Capture {
    // SAFETY: all calls go through live COM interfaces on the worker thread.
    unsafe {
        let focused = a.GetFocusedElement().ok();
        cap.sensitive = focused
            .as_ref()
            .and_then(|f| f.CurrentIsPassword().ok())
            .is_some_and(|b| b.as_bool());
        // The site is read even with context off: domain exclusions need it.
        if is_browser(&cap.process) {
            cap.host = read_host(a, HWND(hwnd as *mut _));
            let (content, terminal) = classify(&cap.process, cap.host.as_deref());
            cap.content = Some(content);
            cap.terminal = terminal;
            cap.excluded = is_excluded(&cap.process, cap.host.as_deref(), apps, domains);
        }
        if cap.sensitive || cap.excluded || !read_context {
            return cap;
        }
        // The focus must still be in the window identified above, or its
        // text would be sent under another window's identity.
        let Some(focused) =
            focused.filter(|f| f.CurrentProcessId().is_ok_and(|p| p as u32 == owner))
        else {
            return cap;
        };
        cap.field = read_field(&focused);
        let mut texts = page_texts(a, &focused);
        cap.page_chars = texts.iter().map(|t| t.chars().count() as u32).sum();
        if let Some(f) = &cap.field {
            texts.push(format!("{}{}{}", f.before, f.selection, f.after));
        }
        cap.names = names_near_emails(&texts);
    }
    cap
}

/// F21: the browser's address bar value, unless the user is typing in it.
unsafe fn read_host(a: &IUIAutomation, hwnd: HWND) -> Option<String> {
    let root = a.ElementFromHandle(hwnd).ok()?;
    let edit = a
        .CreatePropertyCondition(
            UIA_ControlTypePropertyId,
            &VARIANT::from(UIA_EditControlTypeId.0),
        )
        .ok()?;
    let mut names: Option<IUIAutomationCondition> = None;
    for name in ADDRESS_BAR_NAMES {
        let c = a
            .CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(*name))
            .ok()?;
        names = Some(match names {
            Some(prev) => a.CreateOrCondition(&prev, &c).ok()?,
            None => c,
        });
    }
    let bar = root
        .FindFirst(
            TreeScope_Descendants,
            &a.CreateAndCondition(&edit, &names?).ok()?,
        )
        .ok()?;
    if bar.CurrentHasKeyboardFocus().is_ok_and(|b| b.as_bool()) {
        return None;
    }
    let value = bar
        .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
        .ok()?
        .CurrentValue()
        .ok()?;
    normalize_host(&value.to_string())
}

/// F23, F24: text around the caret, or the whole value when the caret is unknown.
unsafe fn read_field(focused: &IUIAutomationElement) -> Option<Field> {
    let value = focused
        .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
        .ok()
        .and_then(|v| v.CurrentValue().ok())
        .map(|b| normalize_newlines(&b.to_string()));
    let name = bstr(focused.CurrentName());
    let help = bstr(focused.CurrentHelpText());
    // Chrome: an empty field has Value "" while TextPattern may return an artifact.
    if value
        .as_deref()
        .is_some_and(|v| v.is_empty() || is_placeholder(v, &name, &help))
    {
        return Some(Field {
            caret_known: true,
            ..Default::default()
        });
    }
    if let Some(field) = read_caret(focused) {
        let whole = format!("{}{}{}", field.before, field.selection, field.after);
        if is_placeholder(&whole, &name, &help) {
            return Some(Field {
                caret_known: true,
                ..Default::default()
            });
        }
        return Some(field);
    }
    value.map(|v| Field {
        before: v,
        ..Default::default()
    })
}

unsafe fn read_caret(focused: &IUIAutomationElement) -> Option<Field> {
    let tp = focused
        .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
        .ok()?;
    let ranges = tp.GetSelection().ok()?;
    if ranges.Length().ok()? < 1 {
        return None;
    }
    let sel = ranges.GetElement(0).ok()?;
    let before = sel.Clone().ok()?;
    before
        .MoveEndpointByRange(
            TextPatternRangeEndpoint_End,
            &sel,
            TextPatternRangeEndpoint_Start,
        )
        .ok()?;
    before
        .MoveEndpointByUnit(
            TextPatternRangeEndpoint_Start,
            TextUnit_Character,
            -FIELD_CHARS,
        )
        .ok()?;
    let after = sel.Clone().ok()?;
    after
        .MoveEndpointByRange(
            TextPatternRangeEndpoint_Start,
            &sel,
            TextPatternRangeEndpoint_End,
        )
        .ok()?;
    after
        .MoveEndpointByUnit(
            TextPatternRangeEndpoint_End,
            TextUnit_Character,
            FIELD_CHARS,
        )
        .ok()?;
    // Chrome's ranges run across the whole page: keep them inside this field.
    let field = tp.DocumentRange().ok()?;
    if before
        .CompareEndpoints(
            TextPatternRangeEndpoint_Start,
            &field,
            TextPatternRangeEndpoint_Start,
        )
        .is_ok_and(|c| c < 0)
    {
        before
            .MoveEndpointByRange(
                TextPatternRangeEndpoint_Start,
                &field,
                TextPatternRangeEndpoint_Start,
            )
            .ok()?;
    }
    if after
        .CompareEndpoints(
            TextPatternRangeEndpoint_End,
            &field,
            TextPatternRangeEndpoint_End,
        )
        .is_ok_and(|c| c > 0)
    {
        after
            .MoveEndpointByRange(
                TextPatternRangeEndpoint_End,
                &field,
                TextPatternRangeEndpoint_End,
            )
            .ok()?;
    }
    Some(Field {
        before: normalize_newlines(&bstr(before.GetText(-1))),
        after: normalize_newlines(&bstr(after.GetText(-1))),
        selection: normalize_newlines(&bstr(sel.GetText(FIELD_CHARS))),
        caret_known: true,
    })
}

/// F26, F32: page text near the field, node by node (a zone holding a
/// password field is never read in bulk), nearest ancestors first, bounded.
/// Only the page `Document` is read: no tabs, bookmarks, profile or buttons.
unsafe fn page_texts(a: &IUIAutomation, focused: &IUIAutomationElement) -> Vec<String> {
    let Ok(walker) = a.ControlViewWalker() else {
        return Vec::new();
    };
    // Ancestors up to the page Document; none → no page text.
    let mut chain = Vec::new();
    let mut current = focused.clone();
    let mut in_document = false;
    for _ in 0..12 {
        let Ok(parent) = walker.GetParentElement(&current) else {
            break;
        };
        chain.push(parent.clone());
        if parent
            .CurrentControlType()
            .is_ok_and(|t| t == UIA_DocumentControlTypeId)
        {
            in_document = true;
            break;
        }
        current = parent;
    }
    if !in_document {
        return Vec::new();
    }
    let started = Instant::now();
    let (mut texts, mut nodes, mut chars) = (Vec::new(), 0usize, 0usize);
    let mut done: Option<IUIAutomationElement> = Some(focused.clone());
    for ancestor in &chain {
        let mut queue = VecDeque::new();
        let mut child = walker.GetFirstChildElement(ancestor).ok();
        while let Some(c) = child {
            let seen = done
                .as_ref()
                .is_some_and(|d| a.CompareElements(&c, d).is_ok_and(|b| b.as_bool()));
            if !seen {
                queue.push_back(c.clone());
            }
            child = walker.GetNextSiblingElement(&c).ok();
        }
        while let Some(el) = queue.pop_front() {
            if nodes >= PAGE_NODES || chars >= PAGE_CHARS || started.elapsed() > PAGE_BUDGET {
                return texts;
            }
            nodes += 1;
            if el.CurrentIsPassword().is_ok_and(|b| b.as_bool()) {
                continue;
            }
            let kind = el.CurrentControlType().unwrap_or_default();
            if kind == UIA_TextControlTypeId
                || kind == UIA_HyperlinkControlTypeId
                || kind == UIA_ListItemControlTypeId
                || kind == UIA_ButtonControlTypeId
            {
                let name = bstr(el.CurrentName());
                if !name.trim().is_empty() {
                    chars += name.chars().count();
                    texts.push(name);
                }
            }
            let mut child = walker.GetFirstChildElement(&el).ok();
            while let Some(c) = child {
                queue.push_back(c.clone());
                child = walker.GetNextSiblingElement(&c).ok();
            }
        }
        done = Some(ancestor.clone());
    }
    texts
}

#[cfg(test)]
mod tests {
    /// F34/S21: a reader stuck past its deadline is dropped and the next read
    /// gets a fresh one.
    #[test]
    fn a_stuck_reader_is_replaced() {
        use std::time::Duration;
        let stuck = super::run(Duration::from_millis(100), |_| {
            std::thread::sleep(Duration::from_millis(1_500));
            1
        });
        assert_eq!(stuck, None);
        assert_eq!(super::run(Duration::from_millis(1_000), |_| 2), Some(2));
    }

    /// Manual check: `cargo test --release --lib context::windows -- --ignored --nocapture`,
    /// then focus a field within 3 s.
    #[test]
    #[ignore]
    fn reads_the_focused_field() {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let cap = super::capture(true, &[], &[]);
        println!("{}", serde_json::to_string_pretty(&cap).unwrap());
        println!("{:?}", super::super::llm_context(&cap));
    }
}
