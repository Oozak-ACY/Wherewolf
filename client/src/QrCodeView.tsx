import { useEffect, useMemo } from "react";
import { qrCode } from "./qrCode";
import { fr } from "./strings";

/** The join link as a QR code filling the Host's screen; one tap anywhere closes it. */
export function QrCodeView(props: { link: string; onClose: () => void }) {
  const { link, onClose } = props;
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
      <svg
        viewBox={`0 0 ${qr.size} ${qr.size}`}
        role="img"
        aria-label={fr.lobby.qrCodeLabel}
        shapeRendering="crispEdges"
      >
        <rect width={qr.size} height={qr.size} fill="#fff" />
        <path d={qr.path} fill="#000" />
      </svg>
      <span>{fr.lobby.qrCodeHint}</span>
    </button>
  );
}
