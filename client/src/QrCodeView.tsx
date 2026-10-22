import { useEffect, useMemo, useRef } from "react";
import { qrCode } from "./qrCode";
import { fr } from "./strings";

/** The join link as a QR code on a parchment sheet over the Host's screen that one tap anywhere closes */
export function QrCodeView(props: { link: string; code: string; onClose: () => void }) {
  const { link, code, onClose } = props;
  const qr = useMemo(() => qrCode(link), [link]);
  const dialog = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    dialog.current?.showModal();
  }, []);

  return (
    <dialog ref={dialog} className="qr-screen" aria-label={fr.lobby.qrCodeLabel} onClose={onClose}>
      <button className="qr-close" onClick={() => dialog.current?.close()} autoFocus>
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
    </dialog>
  );
}
