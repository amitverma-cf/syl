export interface SpinnerProps {
  label?: string;
  size?: number;
}

function Spinner({ label = "Loading…", size = 14 }: SpinnerProps) {
  return (
    <span className="ui-spinner-row" role="status">
      <span
        className="ui-spinner"
        style={{ width: size, height: size }}
        aria-hidden
      />
      <span className="ui-spinner-label">{label}</span>
    </span>
  );
}

export default Spinner;
