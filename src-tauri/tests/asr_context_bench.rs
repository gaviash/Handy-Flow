//! Handy Adaptive (spec §7): does sending context to an ASR model help?
//! Replays real dictations on each model with and without context and prints
//! WER, learned-term accuracy and latency. Run by hand:
//!
//! ```text
//! HANDY_ASR_MODELS="C:/…/parakeet.gguf;C:/…/qwen3-asr.gguf" \
//! HANDY_ASR_CASES="C:/…/scratchpad/asr-cases.json" \
//! cargo test --test asr_context_bench -- --ignored --nocapture
//! ```
//!
//! `asr-cases.json` = `[{ "wav": "path", "expected": "text", "terms": ["log"] }]`,
//! built from the user's own recordings by `adaptive/bench/asr-cases.ps1`
//! (never committed: real dictations).

use std::time::Instant;
use transcribe_cpp::{Feature, Model, RunExtension, RunOptions, WhisperRunOptions};

#[derive(serde::Deserialize)]
struct Case {
    wav: String,
    expected: String,
    #[serde(default)]
    terms: Vec<String>,
}

#[test]
fn wer_counts_word_edits() {
    assert_eq!(wer("le log du serveur", "le gag du serveur"), 0.25);
    assert_eq!(wer("Le log, du serveur.", "le log du serveur"), 0.0);
    assert_eq!(wer("", ""), 0.0);
    assert_eq!(
        terms_ok(&["log".into(), "Claude".into()], "le log de claude"),
        2
    );
}

#[test]
#[ignore = "needs local models and recordings; run by hand"]
fn asr_context_bench() {
    let models = std::env::var("HANDY_ASR_MODELS").expect("HANDY_ASR_MODELS");
    let cases: Vec<Case> = serde_json::from_str(
        &std::fs::read_to_string(std::env::var("HANDY_ASR_CASES").expect("HANDY_ASR_CASES"))
            .expect("read cases"),
    )
    .expect("parse cases");
    let audio: Vec<Vec<f32>> = cases.iter().map(|c| read_wav(&c.wav)).collect();
    // Dynamic-backend builds load their CPU/Vulkan modules from the staged libs.
    transcribe_cpp::init_backends(concat!(env!("CARGO_MANIFEST_DIR"), "/transcribe-libs"))
        .expect("init backends");
    println!("model | features | wer_without | wer_with | terms_without | terms_with | ms_without | ms_with");
    for path in models.split(';').filter(|p| !p.trim().is_empty()) {
        let model = Model::load(path.trim()).expect("load model");
        let mut session = model.session().expect("session");
        let caps = (
            model.supports(Feature::Vocabulary),
            model.supports(Feature::ContextPrompt),
            model.supports(Feature::InitialPrompt),
        );
        let mut stats = [(0.0, 0usize, 0u128), (0.0, 0usize, 0u128)];
        let total_terms: usize = cases.iter().map(|c| c.terms.len()).sum();
        for (case, pcm) in cases.iter().zip(&audio) {
            for (i, with) in [false, true].into_iter().enumerate() {
                let options = if with {
                    with_context(caps, &case.terms)
                } else {
                    RunOptions::default()
                };
                let started = Instant::now();
                let text = session.run(pcm, &options).expect("run").text;
                stats[i].2 += started.elapsed().as_millis();
                stats[i].0 += wer(&case.expected, &text);
                stats[i].1 += terms_ok(&case.terms, &text);
            }
        }
        let n = cases.len().max(1) as f64;
        println!(
            "{} | vocab={} prompt={} initial={} | {:.3} | {:.3} | {}/{} | {}/{} | {} | {}",
            path.rsplit(['/', '\\']).next().unwrap_or(path),
            caps.0,
            caps.1,
            caps.2,
            stats[0].0 / n,
            stats[1].0 / n,
            stats[0].1,
            total_terms,
            stats[1].1,
            total_terms,
            stats[0].2,
            stats[1].2
        );
    }
}

/// The context each capability takes; the Whisper initial prompt never goes
/// with `vocabulary`/`prompt` (transcribe-cpp rejects the mix).
fn with_context(caps: (bool, bool, bool), terms: &[String]) -> RunOptions {
    let (vocabulary, context_prompt, initial_prompt) = caps;
    let prompt = format!("Vocabulaire : {}.", terms.join(", "));
    let mut o = RunOptions::default();
    if initial_prompt {
        o.family = Some(RunExtension::Whisper(WhisperRunOptions {
            initial_prompt: Some(prompt),
            ..Default::default()
        }));
        return o;
    }
    if vocabulary {
        o.vocabulary = terms.to_vec();
    }
    if context_prompt {
        o.prompt = Some(prompt);
    }
    o
}

/// Letters and digits, lowercased (as `learning::key`).
fn words(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|w| {
            w.chars()
                .filter(|c| c.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// Word error rate: word edits / reference words.
fn wer(reference: &str, hypothesis: &str) -> f64 {
    let (r, h) = (words(reference), words(hypothesis));
    if r.is_empty() {
        return if h.is_empty() { 0.0 } else { 1.0 };
    }
    let mut row: Vec<usize> = (0..=h.len()).collect();
    for i in 1..=r.len() {
        let mut prev = row[0];
        row[0] = i;
        for j in 1..=h.len() {
            let cur = row[j];
            row[j] = (prev + usize::from(r[i - 1] != h[j - 1]))
                .min(row[j] + 1)
                .min(row[j - 1] + 1);
            prev = cur;
        }
    }
    row[h.len()] as f64 / r.len() as f64
}

/// How many expected terms the transcript writes (case-insensitive).
fn terms_ok(terms: &[String], text: &str) -> usize {
    let said = words(text).join(" ");
    terms
        .iter()
        .filter(|t| {
            let t = words(t).join(" ");
            !t.is_empty() && format!(" {said} ").contains(&format!(" {t} "))
        })
        .count()
}

fn read_wav(path: &str) -> Vec<f32> {
    let mut r = hound::WavReader::open(path).expect("open wav");
    let spec = r.spec();
    assert_eq!(spec.sample_rate, 16_000, "{path}: 16 kHz expected");
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().map(Result::unwrap).collect(),
        hound::SampleFormat::Int => {
            let scale = (1i64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>()
                .map(|s| s.unwrap() as f32 / scale)
                .collect()
        }
    };
    // Mono: keep the first channel.
    samples
        .into_iter()
        .step_by(spec.channels as usize)
        .collect()
}
