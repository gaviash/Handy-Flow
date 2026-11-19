import React, { useState } from "react";
import { Input } from "../../ui/Input";

interface BaseUrlFieldProps {
  value: string;
  onBlur: (value: string) => void;
  disabled: boolean;
  placeholder?: string;
  className?: string;
  id?: string;
}

export const BaseUrlField: React.FC<BaseUrlFieldProps> = React.memo(
  ({ value, onBlur, disabled, placeholder, className = "", id }) => {
    const [localValue, setLocalValue] = useState(value);

    // Sync with prop changes
    React.useEffect(() => {
      setLocalValue(value);
    }, [value]);

    return (
      <Input
        id={id}
        type="text"
        value={localValue}
        onChange={(event) => setLocalValue(event.target.value)}
        onBlur={() => onBlur(localValue)}
        placeholder={placeholder}
        variant="compact"
        disabled={disabled}
        className={`flex-1 min-w-[360px] ${className}`}
      />
    );
  },
);

BaseUrlField.displayName = "BaseUrlField";
