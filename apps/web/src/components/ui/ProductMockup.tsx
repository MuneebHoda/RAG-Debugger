import styles from "./ProductMockup.module.css";

type ProductMockupProps = {
  src: string;
  alt: string;
  label?: string;
  elevated?: boolean;
};

export function ProductMockup({
  src,
  alt,
  label,
  elevated = false,
}: ProductMockupProps) {
  return (
    <figure
      className={`${styles.mockup} ${elevated ? styles.elevated : ""}`}
      aria-label={label}
    >
      <img src={src} alt={alt} />
      {label ? <figcaption>{label}</figcaption> : null}
    </figure>
  );
}
