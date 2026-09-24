// The Narrator's voice is the browser's speech synthesis.

/**
 * Must run synchronously inside the "Rejoindre" tap. iOS only lets a page speak
 * later, without a tap, if it already spoke once during a user gesture
 * (validated on real phones, see issue #1).
 */
export function unlockSpeech(): void {
  if (!("speechSynthesis" in window)) return;
  window.speechSynthesis.speak(new SpeechSynthesisUtterance(""));
}
