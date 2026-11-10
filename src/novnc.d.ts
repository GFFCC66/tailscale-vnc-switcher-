declare module "@novnc/novnc" {
  export default class RFB {
    constructor(
      target: HTMLElement,
      url: string,
      options?: {
        credentials?: { password?: string; username?: string };
        shared?: boolean;
        wsProtocols?: string[];
      },
    );
    viewOnly: boolean;
    scaleViewport: boolean;
    clipViewport: boolean;
    resizeSession: boolean;
    showDotCursor: boolean;
    background: string;
    qualityLevel: number;
    compressionLevel: number;
    focus(options?: { preventScroll?: boolean }): void;
    blur(): void;
    disconnect(): void;
    sendCredentials(creds: { password?: string }): void;
    addEventListener(type: string, listener: (event: CustomEvent) => void): void;
    removeEventListener(type: string, listener: (event: CustomEvent) => void): void;
  }
}
