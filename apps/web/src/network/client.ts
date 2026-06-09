import type { ClientEvent, ServerEvent } from "@128/protocol";

export interface GameConnection {
  ready: Promise<void>;
  close(): void;
  send(event: ClientEvent): void;
}

interface TransportState {
  datagramWriter?: WritableStreamDefaultWriter<Uint8Array>;
}

export function connectToGame(
  url: string,
  onEvent: (event: ServerEvent) => void,
): GameConnection {
  const transport = new WebTransport(url, createTransportOptions());
  const state = initializeTransport(transport, onEvent);

  void transport.closed.catch((error: unknown) => {
    console.error("WebTransport connection closed unexpectedly", error);
  });

  return {
    ready: state.then(() => undefined),
    close: () => transport.close(),
    send: (event) => {
      void sendEvent(transport, state, event).catch(reportNetworkError);
    },
  };
}

async function initializeTransport(
  transport: WebTransport,
  onEvent: (event: ServerEvent) => void,
): Promise<TransportState> {
  await transport.ready;
  const datagrams = getDatagrams(transport);

  void sendReliable(
    transport,
    encodeClientEvent({
      type: "transport-capabilities",
      datagrams: datagrams !== undefined,
    }),
  ).catch(reportNetworkError);
  if (datagrams) {
    void readDatagrams(datagrams, onEvent).catch(reportNetworkError);
  }
  void readReliableEvents(transport, onEvent).catch(reportNetworkError);

  return datagrams ? { datagramWriter: datagrams.writable.getWriter() } : {};
}

async function sendEvent(
  transport: WebTransport,
  statePromise: Promise<TransportState>,
  event: ClientEvent,
): Promise<void> {
  const state = await statePromise;
  const payload = encodeClientEvent(event);

  if (event.type === "pointer-move" && state.datagramWriter) {
    await state.datagramWriter.write(payload);
    return;
  }

  await sendReliable(transport, payload);
}

async function sendReliable(
  transport: WebTransport,
  payload: Uint8Array,
): Promise<void> {
  const stream = await transport.createUnidirectionalStream();
  const writer = stream.getWriter();

  await writer.write(payload);
  await writer.close();
}

async function readDatagrams(
  datagrams: WebTransportDatagramDuplexStream,
  onEvent: (event: ServerEvent) => void,
): Promise<void> {
  const reader = datagrams.readable.getReader();

  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      return;
    }
    onEvent(decodeServerEvent(value));
  }
}

async function readReliableEvents(
  transport: WebTransport,
  onEvent: (event: ServerEvent) => void,
): Promise<void> {
  const reader = transport.incomingUnidirectionalStreams.getReader();

  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      return;
    }
    onEvent(decodeServerEvent(await readStream(value)));
  }
}

async function readStream(stream: ReadableStream<Uint8Array>): Promise<Uint8Array> {
  const reader = stream.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;

  while (true) {
    const { done, value } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(value);
    length += value.byteLength;
  }

  const payload = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) {
    payload.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return payload;
}

function decodeServerEvent(payload: Uint8Array): ServerEvent {
  return JSON.parse(new TextDecoder().decode(payload)) as ServerEvent;
}

function encodeClientEvent(event: ClientEvent): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(event));
}

function getDatagrams(transport: WebTransport): WebTransportDatagramDuplexStream | undefined {
  return (transport as WebTransport & { datagrams?: WebTransportDatagramDuplexStream }).datagrams;
}

function reportNetworkError(error: unknown): void {
  console.error("WebTransport operation failed", error);
}

function createTransportOptions(): WebTransportOptions {
  const hash = import.meta.env.VITE_WEBTRANSPORT_CERT_HASH;
  if (!hash) {
    return {};
  }

  return {
    serverCertificateHashes: [
      {
        algorithm: "sha-256",
        value: parseCertificateHash(hash),
      },
    ],
  };
}

function parseCertificateHash(hash: string): ArrayBuffer {
  const parts = hash.split(":");
  if (parts.length !== 32 || parts.some((part) => !/^[0-9a-f]{2}$/i.test(part))) {
    throw new Error("VITE_WEBTRANSPORT_CERT_HASH must be a colon-separated SHA-256 hash");
  }
  return Uint8Array.from(parts, (part) => Number.parseInt(part, 16)).buffer;
}
