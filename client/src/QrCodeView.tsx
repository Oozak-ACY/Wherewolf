import { useEffect, useMemo } from "react";
import { qrCode } from "./qrCode";
import { fr } from "./strings";

/** The join link as a QR code on a parchment sheet over the Host's screen; one tap anywhere closes it. */
export function QrCodeView(props: { link: string; code: string; onClose: () => void }) {
  const { link, code, onClose } = props;
  const qr = useMemo(() => qrCode(link), [link]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <button className="qr-screen" onClick={onClose} autoFocus>
      <span className="qr-sheet">
        <span className="qr-code">{code}</span>
        <svg
          viewBox={`0 0 ${qr.size} ${qr.size}`}
          role="img"
          aria-label={fr.lobby.qrCodeLabel}
          shapeRendering="crispEdges"
        >
          <rect width={qr.size} height={qr.size} fill="#fff" />
          <path d={qr.path} fill="#000" />
        </svg>
        <span className="qr-hint">{fr.lobby.qrCodeHint}</span>
      </span>
    </button>
  );
}
