import { X } from "lucide-react";
import type { ReactNode } from "react";

interface ModalProps {
  title: string;
  subtitle?: string;
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  wide?: boolean;
}

export function Modal({ title, subtitle, open, onClose, children, wide }: ModalProps) {
  if (!open) return null;
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className={`modal ${wide ? "modal-wide" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="modal-header">
          <div>
            <h2>{title}</h2>
            {subtitle && <p>{subtitle}</p>}
          </div>
          <button className="icon-button" onClick={onClose} aria-label="Fermer">
            <X size={20} />
          </button>
        </header>
        <div className="modal-content">{children}</div>
      </section>
    </div>
  );
}
