import React from "react";

interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  id: string;
  "aria-label"?: string;
  disabled?: boolean;
}

/**
 * Reusable switch-style toggle. Renders a single underlying checkbox and a
 * styled slider, so it stays keyboard-accessible while looking like a switch.
 */
export const Toggle: React.FC<ToggleProps> = ({
  checked,
  onChange,
  id,
  disabled = false,
  ...rest
}) => {
  return (
    <label className={`switch-toggle ${disabled ? "disabled" : ""}`}>
      <input
        id={id}
        type="checkbox"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
        aria-label={rest["aria-label"]}
      />
      <span className={`slider ${checked ? "on" : ""}`} />
    </label>
  );
};
