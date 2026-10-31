import {
  encodeInput,
  PadButton,
  readServerMessage,
} from "../sinteract/web/src/protocol.ts";
import { decodeScene, type Element } from "../sinteract/web/src/scene.ts";

type RoomStatus = "preparing" | "waiting" | "running" | "failed";

interface RoomSnapshot {
  code: string;
  status: RoomStatus;
  players: Array<{ id: number; nickname: string }>;
}

interface CreatedRoom {
  code: string;
  host_token: string;
}

interface TextMessage {
  type: string;
  room?: RoomSnapshot;
  message?: string;
}

interface SocketMessage {
  receivedAt: number;
  data: string | ArrayBuffer;
}

interface ProcessSample {
  elapsed_seconds: number;
  processes: number;
  cpu_percent: number;
  rss_mib: number;
}

const baseUrl = env("BASE_URL", "http://127.0.0.1:3100").replace(/\/$/, "");
const gameId = requiredPositiveInteger("GAME_ID");
const preparationRuns = positiveInteger("PREPARATION_RUNS", 30);
const latencyRuns = positiveInteger("LATENCY_RUNS", 30);
const capacityRooms = positiveInteger("CAPACITY_ROOMS", 16);
const capacitySeconds = positiveInteger("CAPACITY_SECONDS", 600);
const sampleSeconds = positiveInteger("SAMPLE_SECONDS", 10);
const output = Deno.env.get("OUTPUT");
const serverPid = optionalPositiveInteger("SERVER_PID");
const timeoutMs = positiveInteger("TIMEOUT_MS", 20_000);

class RoomSocket {
  readonly socket: WebSocket;
  readonly opened: Promise<void>;
  frames = 0;
  closedUnexpectedly = false;
  error: string | null = null;

  #queue: SocketMessage[] = [];
  #waiters: Array<(message: SocketMessage) => void> = [];
  #drainBinary = false;
  #closing = false;

  constructor(url: string) {
    this.socket = new WebSocket(url, "sarcade.room.v1");
    this.socket.binaryType = "arraybuffer";
    this.opened = new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`timeout ao abrir ${url}`)),
        timeoutMs,
      );
      this.socket.onopen = () => {
        clearTimeout(timer);
        resolve();
      };
      this.socket.onerror = () => {
        this.error = "erro no WebSocket";
        clearTimeout(timer);
        reject(new Error(`erro ao abrir ${url}`));
      };
    });
    this.socket.onmessage = (event) => {
      if (event.data instanceof ArrayBuffer) {
        this.frames++;
        if (this.#drainBinary) return;
      }
      const message = { receivedAt: performance.now(), data: event.data };
      const waiter = this.#waiters.shift();
      if (waiter) waiter(message);
      else this.#queue.push(message);
    };
    this.socket.onclose = () => {
      if (!this.#closing) this.closedUnexpectedly = true;
    };
  }

  drainBinary(): void {
    this.#drainBinary = true;
    this.#queue = this.#queue.filter((message) =>
      typeof message.data === "string"
    );
  }

  send(data: string | ArrayBuffer): void {
    this.socket.send(data);
  }

  async next(
    predicate: (message: SocketMessage) => boolean,
  ): Promise<SocketMessage> {
    const deadline = performance.now() + timeoutMs;
    while (true) {
      for (let index = 0; index < this.#queue.length; index++) {
        const message = this.#queue[index];
        if (predicate(message)) {
          this.#queue.splice(index, 1);
          return message;
        }
      }
      const remaining = deadline - performance.now();
      if (remaining <= 0) throw new Error("timeout esperando mensagem da sala");
      const message = await Promise.race([
        new Promise<SocketMessage>((resolve) => this.#waiters.push(resolve)),
        delay(remaining).then(() => {
          throw new Error("timeout esperando mensagem da sala");
        }),
      ]);
      if (predicate(message)) return message;
      this.#queue.push(message);
    }
  }

  async text(type: string): Promise<TextMessage> {
    const message = await this.next((candidate) => {
      if (typeof candidate.data !== "string") return false;
      return (JSON.parse(candidate.data) as TextMessage).type === type;
    });
    return JSON.parse(message.data as string) as TextMessage;
  }

  close(): void {
    this.#closing = true;
    this.socket.close();
  }
}

async function main(): Promise<void> {
  progress("Conferindo a instância e medindo o RTT HTTP");
  await expectOk(`${baseUrl}/health`);
  const httpRoundTripMs: number[] = [];
  for (let run = 0; run < 30; run++) {
    const started = performance.now();
    await expectOk(`${baseUrl}/health`);
    httpRoundTripMs.push(performance.now() - started);
  }

  progress(`Medindo preparação (${preparationRuns} repetições)`);
  const preparationMs: number[] = [];
  for (let run = 0; run < preparationRuns; run++) {
    const started = performance.now();
    const room = await createRoom(`Preparacao-${run + 1}`);
    await waitForStatus(room.code, "waiting");
    preparationMs.push(performance.now() - started);
    await removeWaitingRoom(room, `Preparacao-${run + 1}`);
    progress(`Preparação ${run + 1}/${preparationRuns}`);
  }

  const inputToFrameMs: number[] = [];
  progress(`Iniciando ${capacityRooms} salas simultâneas`);
  const capacity: RunningRoom[] = [];
  const capacityFailures: string[] = [];
  try {
    for (let room = 0; room < capacityRooms; room++) {
      try {
        const active = await createRunningRoom(`Capacidade-${room + 1}`);
        active.host.drainBinary();
        active.guest.drainBinary();
        capacity.push(active);
        progress(`Sala simultânea ${room + 1}/${capacityRooms}`);
      } catch (error) {
        capacityFailures.push(`sala ${room + 1}: ${String(error)}`);
        break;
      }
    }

    const samples: ProcessSample[] = [];
    const capacityStarted = performance.now();
    let nextSampleAt = sampleSeconds;
    while ((performance.now() - capacityStarted) / 1000 < capacitySeconds) {
      const elapsedBeforeDelay = (performance.now() - capacityStarted) / 1000;
      await delay(
        Math.min(1000, (capacitySeconds - elapsedBeforeDelay) * 1000),
      );
      for (const room of capacity) {
        for (const socket of [room.host, room.guest]) {
          if (socket.socket.readyState !== WebSocket.OPEN) continue;
          socket.send(encodeInput({
            kind: "pad",
            action: { kind: "down", button: PadButton.B },
          }));
          socket.send(encodeInput({
            kind: "pad",
            action: { kind: "up", button: PadButton.B },
          }));
        }
      }
      const elapsed = (performance.now() - capacityStarted) / 1000;
      if (elapsed >= nextSampleAt || elapsed >= capacitySeconds) {
        if (serverPid !== null) {
          samples.push(await processSample(serverPid, elapsed));
        }
        progress(
          `Capacidade: ${
            Math.min(elapsed, capacitySeconds).toFixed(0)
          }/${capacitySeconds} s`,
        );
        nextSampleAt += sampleSeconds;
      }
    }

    for (const [index, room] of capacity.entries()) {
      const status = await roomStatus(room.code).catch((error) => {
        capacityFailures.push(`sala ${index + 1}: ${String(error)}`);
        return null;
      });
      if (status !== "running") {
        capacityFailures.push(`sala ${index + 1}: estado final ${status}`);
      }
      for (const socket of [room.host, room.guest]) {
        if (socket.closedUnexpectedly || socket.error) {
          capacityFailures.push(
            `sala ${index + 1}: conexão encerrada (${
              socket.error ?? "sem erro informado"
            })`,
          );
        }
      }
    }

    for (const room of capacity) room.close();

    progress(`Medindo entrada até frame (${latencyRuns} repetições)`);
    let running: RunningRoom | null = null;
    try {
      for (let run = 0; run < latencyRuns; run++) {
        if (run % 4 === 0) {
          running?.close();
          running = await createRunningRoom(
            `Latencia-${Math.floor(run / 4) + 1}`,
          );
        }
        if (!running) throw new Error("sala de latência não foi criada");
        const expectedScore = run % 4 + 1;
        const started = performance.now();
        running.host.send(encodeInput({
          kind: "pad",
          action: { kind: "down", button: PadButton.A },
        }));
        const frame = await running.host.next((message) =>
          typeof message.data !== "string" &&
          frameHasScore(message.data, 1, expectedScore)
        );
        inputToFrameMs.push(frame.receivedAt - started);
        running.host.send(encodeInput({
          kind: "pad",
          action: { kind: "up", button: PadButton.A },
        }));
        progress(`Latência ${run + 1}/${latencyRuns}`);
      }
    } finally {
      running?.close();
    }

    const result = {
      schema: "sarcade.rnf08-benchmark.v1",
      measured_at: new Date().toISOString(),
      configuration: {
        base_url: baseUrl,
        game_id: gameId,
        preparation_runs: preparationRuns,
        latency_runs: latencyRuns,
        capacity_rooms_requested: capacityRooms,
        players_per_room: 2,
        capacity_seconds: capacitySeconds,
        process_sample_seconds: sampleSeconds,
        server_pid: serverPid,
      },
      environment: await environment(),
      http_round_trip_ms: summarize(httpRoundTripMs),
      room_preparation_ms: {
        ...summarize(preparationMs),
        first: round(preparationMs[0]),
        warm: summarize(preparationMs.slice(1)),
      },
      input_to_frame_arrival_ms: summarize(inputToFrameMs),
      capacity: {
        rooms_started: capacity.length,
        players_connected: capacity.length * 2,
        duration_seconds: capacitySeconds,
        failures: capacityFailures,
        frames_received: capacity.reduce(
          (sum, room) => sum + room.host.frames + room.guest.frames,
          0,
        ),
        process_samples: samples,
        process_summary: summarizeProcessSamples(samples),
      },
      notes: [
        "Preparação: POST /api/rooms até o primeiro GET com status waiting.",
        "Latência: envio de PadDown até a chegada do primeiro frame cuja cena contém o novo placar; rasterização no canvas não está incluída.",
        "RTT HTTP: GET /health na mesma rota de rede usada pelo ensaio.",
        "Durante a capacidade, cada conexão envia PadDown e PadUp de uma ação sem efeito no placar uma vez por segundo.",
        "CPU e RSS abrangem o processo Sarcade e seus descendentes; CPU é a média desde o início informada por ps, e 100% equivale a um núcleo lógico.",
        "Percentil 95 calculado pelo método nearest-rank.",
      ],
    };

    const json = `${JSON.stringify(result, null, 2)}\n`;
    if (output) {
      await Deno.writeTextFile(output, json);
      progress(`Resultado gravado em ${output}`);
    } else {
      console.log(json);
    }
  } finally {
    for (const room of capacity) room.close();
  }
}

interface RunningRoom {
  code: string;
  host: RoomSocket;
  guest: RoomSocket;
  close(): void;
}

async function createRunningRoom(label: string): Promise<RunningRoom> {
  const created = await createRoom(`${label}-host`);
  await waitForStatus(created.code, "waiting");
  const host = await openRoomSocket(
    created.code,
    `${label}-host`,
    created.host_token,
  );
  await host.text("joined");
  const guest = await openRoomSocket(created.code, `${label}-convidado`);
  await guest.text("joined");
  await host.next((message) => {
    if (typeof message.data !== "string") return false;
    const parsed = JSON.parse(message.data) as TextMessage;
    return parsed.room?.players.length === 2;
  });
  host.send(JSON.stringify({ type: "start" }));
  await host.text("room_started");
  await guest.text("room_started");
  await Promise.all([waitForAnyFrame(host), waitForAnyFrame(guest)]);
  return {
    code: created.code,
    host,
    guest,
    close() {
      host.close();
      guest.close();
    },
  };
}

async function createRoom(nickname: string): Promise<CreatedRoom> {
  const response = await fetch(`${baseUrl}/api/rooms`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ game_id: gameId, nickname }),
  });
  if (!response.ok) {
    throw new Error(
      `criação da sala: ${response.status} ${await response.text()}`,
    );
  }
  return await response.json() as CreatedRoom;
}

async function waitForStatus(
  code: string,
  expected: RoomStatus,
): Promise<void> {
  const deadline = performance.now() + timeoutMs;
  while (performance.now() < deadline) {
    const status = await roomStatus(code);
    if (status === expected) return;
    if (status === "failed") {
      throw new Error(`sala ${code} falhou durante a preparação`);
    }
    await delay(5);
  }
  throw new Error(`timeout esperando a sala ${code} ficar ${expected}`);
}

async function roomStatus(code: string): Promise<RoomStatus> {
  const response = await fetch(
    `${baseUrl}/api/rooms/${encodeURIComponent(code)}`,
  );
  if (!response.ok) {
    throw new Error(`consulta da sala ${code}: HTTP ${response.status}`);
  }
  return ((await response.json()) as RoomSnapshot).status;
}

async function removeWaitingRoom(
  room: CreatedRoom,
  nickname: string,
): Promise<void> {
  const socket = await openRoomSocket(room.code, nickname, room.host_token);
  await socket.text("joined");
  socket.close();
  await delay(10);
}

async function openRoomSocket(
  code: string,
  nickname: string,
  hostToken?: string,
): Promise<RoomSocket> {
  const url = new URL(baseUrl);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  url.pathname = `/api/rooms/${encodeURIComponent(code)}/ws`;
  url.searchParams.set("nickname", nickname);
  if (hostToken) url.searchParams.set("host_token", hostToken);
  const socket = new RoomSocket(url.toString());
  await socket.opened;
  return socket;
}

async function waitForAnyFrame(socket: RoomSocket): Promise<void> {
  await socket.next((message) => typeof message.data !== "string");
}

function frameHasScore(
  payload: ArrayBuffer,
  player: number,
  score: number,
): boolean {
  const message = readServerMessage(payload);
  if (message?.kind !== "frame") return false;
  const texts = collectText(decodeScene(message.scene).elements);
  return texts.includes(`Jogador ${player}: ${score}`);
}

function collectText(elements: Element[]): string[] {
  const texts: string[] = [];
  for (const element of elements) {
    if (element.kind === "text") texts.push(element.text);
    if (element.kind === "layer" || element.kind === "clipped") {
      texts.push(...collectText(element.elements));
    }
  }
  return texts;
}

async function expectOk(url: string): Promise<void> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  await response.arrayBuffer();
}

async function processSample(
  rootPid: number,
  elapsed: number,
): Promise<ProcessSample> {
  const pids = await processTree(rootPid);
  let rssKiB = 0;
  let cpuPercent = 0;
  const command = new Deno.Command("ps", {
    args: ["-o", "%cpu=", "-p", pids.join(",")],
    stdout: "piped",
    stderr: "null",
  });
  const output = await command.output();
  if (output.success) {
    cpuPercent = new TextDecoder().decode(output.stdout).trim().split(/\s+/)
      .filter(Boolean).reduce((sum, value) => sum + Number(value), 0);
  }
  for (const pid of pids) {
    try {
      const status = await Deno.readTextFile(`/proc/${pid}/status`);
      const match = /^VmRSS:\s+(\d+)\s+kB$/m.exec(status);
      if (match) rssKiB += Number(match[1]);
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) throw error;
    }
  }
  return {
    elapsed_seconds: round(elapsed),
    processes: pids.length,
    cpu_percent: round(cpuPercent),
    rss_mib: round(rssKiB / 1024),
  };
}

async function processTree(rootPid: number): Promise<number[]> {
  const found = new Set<number>();
  const pending = [rootPid];
  while (pending.length > 0) {
    const pid = pending.pop()!;
    if (found.has(pid)) continue;
    found.add(pid);
    try {
      for await (const task of Deno.readDir(`/proc/${pid}/task`)) {
        if (!task.isDirectory) continue;
        const children = await Deno.readTextFile(
          `/proc/${pid}/task/${task.name}/children`,
        ).catch((error) => {
          if (error instanceof Deno.errors.NotFound) return "";
          throw error;
        });
        for (const child of children.trim().split(/\s+/).filter(Boolean)) {
          pending.push(Number(child));
        }
      }
    } catch (error) {
      if (!(error instanceof Deno.errors.NotFound)) throw error;
    }
  }
  return [...found];
}

async function environment(): Promise<Record<string, unknown>> {
  const cpuInfo = await Deno.readTextFile("/proc/cpuinfo").catch(() => "");
  const cpuModel = /^model name\s*:\s*(.+)$/m.exec(cpuInfo)?.[1] ??
    "desconhecido";
  const memory = Deno.systemMemoryInfo();
  return {
    os: Deno.build.os,
    architecture: Deno.build.arch,
    kernel: Deno.osRelease(),
    cpu_model: cpuModel,
    logical_cpus: navigator.hardwareConcurrency,
    total_memory_mib: round(memory.total / 1024 / 1024),
    deno: Deno.version.deno,
    v8: Deno.version.v8,
  };
}

function summarize(values: number[]): Record<string, number> {
  if (values.length === 0) return { count: 0 };
  const sorted = [...values].sort((a, b) => a - b);
  return {
    count: sorted.length,
    min: round(sorted[0]),
    median: round(percentile(sorted, 0.5)),
    p95: round(percentile(sorted, 0.95)),
    max: round(sorted.at(-1)!),
    mean: round(sorted.reduce((sum, value) => sum + value, 0) / sorted.length),
  };
}

function summarizeProcessSamples(
  samples: ProcessSample[],
): Record<string, number> {
  if (samples.length === 0) return { count: 0 };
  return {
    count: samples.length,
    cpu_median_percent:
      summarize(samples.map((sample) => sample.cpu_percent)).median,
    cpu_p95_percent: summarize(samples.map((sample) => sample.cpu_percent)).p95,
    cpu_max_percent: Math.max(...samples.map((sample) => sample.cpu_percent)),
    rss_median_mib: summarize(samples.map((sample) => sample.rss_mib)).median,
    rss_p95_mib: summarize(samples.map((sample) => sample.rss_mib)).p95,
    rss_max_mib: Math.max(...samples.map((sample) => sample.rss_mib)),
    processes_max: Math.max(...samples.map((sample) => sample.processes)),
  };
}

function percentile(sorted: number[], fraction: number): number {
  return sorted[Math.max(0, Math.ceil(sorted.length * fraction) - 1)];
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

function env(name: string, fallback: string): string {
  return Deno.env.get(name) ?? fallback;
}

function positiveInteger(name: string, fallback: number): number {
  const value = Number(Deno.env.get(name) ?? fallback);
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${name} deve ser um inteiro positivo`);
  }
  return value;
}

function optionalPositiveInteger(name: string): number | null {
  const raw = Deno.env.get(name);
  if (raw === undefined) return null;
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${name} deve ser um inteiro positivo`);
  }
  return value;
}

function requiredPositiveInteger(name: string): number {
  const raw = Deno.env.get(name);
  if (raw === undefined) throw new Error(`${name} deve ser informado`);
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${name} deve ser um inteiro positivo`);
  }
  return value;
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function progress(message: string): void {
  console.error(`[RNF08] ${message}`);
}

if (import.meta.main) {
  await main();
}
