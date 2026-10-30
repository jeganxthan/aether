
import { useEffect, useRef, useState } from "react";
import {
  AudioLines,
  Check,
  Mic,
  MicOff,
  Play,
  RefreshCw,
  Speaker,
  Volume2,
  VolumeX,
} from "lucide-react";
import { soundService, type AudioDevices } from "../services/sound";

export default function Sound() {
  const [devices, setDevices] = useState<AudioDevices | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [testing, setTesting] = useState(false);
  const [level, setLevel] = useState(0);
  const [recordingUrl, setRecordingUrl] = useState("");
  const [micMuted, setMicMuted] = useState(false);

  const streamRef = useRef<MediaStream | null>(null);
  const recorderRef = useRef<MediaRecorder | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const rafRef = useRef<number | null>(null);
  const chunksRef = useRef<Blob[]>([]);
  const mountedRef = useRef(true);

  async function refresh() {
    try {
      setError("");
      const result = await soundService.listDevices();
      if (mountedRef.current) {
        setDevices(result);
        const defaultInput = result.inputs.find((d) => d.isDefault);
        setMicMuted(defaultInput?.muted ?? false);
      }
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    mountedRef.current = true;
    void refresh();

    return () => {
      mountedRef.current = false;
      stopTest();
    };
  }, []);

  async function selectOutput(name: string) {
    setBusy(true);
    setError("");
    try {
      setDevices(await soundService.selectOutput(name));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function selectInput(name: string) {
    setBusy(true);
    setError("");
    try {
      setDevices(await soundService.selectInput(name));
      setRecordingUrl("");
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function toggleMicMute() {
    const next = !micMuted;
    setBusy(true);
    setError("");
    try {
      const result = await soundService.setMicrophoneMute(next);
      setDevices((current) =>
        current
          ? {
              ...current,
              inputs: current.inputs.map((device) =>
                device.isDefault ? { ...device, muted: result.muted } : device,
              ),
            }
          : current,
      );
      setMicMuted(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function startTest() {
    if (testing) {
      stopTest();
      return;
    }

    try {
      setError("");
      setRecordingUrl("");
      if (!navigator.mediaDevices?.getUserMedia) {
        throw new Error("Microphone capture is not available in this settings window.");
      }

      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
      });

      streamRef.current = stream;
      chunksRef.current = [];

      const context = new AudioContext();
      audioContextRef.current = context;
      await context.resume();
      const source = context.createMediaStreamSource(stream);
      const analyser = context.createAnalyser();
      analyser.fftSize = 512;
      source.connect(analyser);

      const data = new Uint8Array(analyser.fftSize);
      const measure = () => {
        analyser.getByteTimeDomainData(data);
        let sum = 0;
        for (const value of data) {
          const sample = (value - 128) / 128;
          sum += sample * sample;
        }
        const rms = Math.sqrt(sum / data.length);
        setLevel(Math.min(100, Math.round(rms * 220)));
        rafRef.current = requestAnimationFrame(measure);
      };
      measure();

      const mimeType = [
        "audio/webm;codecs=opus",
        "audio/webm",
        "audio/ogg;codecs=opus",
      ].find((type) => MediaRecorder.isTypeSupported(type));
      const recorder = mimeType
        ? new MediaRecorder(stream, { mimeType })
        : new MediaRecorder(stream);
      recorderRef.current = recorder;

      recorder.ondataavailable = (event) => {
        if (event.data.size) chunksRef.current.push(event.data);
      };

      recorder.onstop = () => {
        if (chunksRef.current.length) {
          const blob = new Blob(chunksRef.current, {
            type: recorder.mimeType || "audio/webm",
          });
          const url = URL.createObjectURL(blob);
          setRecordingUrl((previous) => {
            if (previous) URL.revokeObjectURL(previous);
            return url;
          });
        }
      };

      recorder.start();
      setTesting(true);
    } catch (e) {
      stopTest();
      const reason =
        e instanceof DOMException
          ? `${e.name}${e.message ? `: ${e.message}` : ""}`
          : e instanceof Error
            ? e.message
            : String(e);
      setError(
        `Cannot access the microphone. Check system permissions and the app's microphone permission. ${reason}`,
      );
    }
  }

  function stopTest() {
    if (rafRef.current !== null) cancelAnimationFrame(rafRef.current);
    rafRef.current = null;

    const recorder = recorderRef.current;
    recorderRef.current = null;
    if (recorder && recorder.state !== "inactive") recorder.stop();

    streamRef.current?.getTracks().forEach((track) => track.stop());
    streamRef.current = null;

    const context = audioContextRef.current;
    audioContextRef.current = null;
    if (context && context.state !== "closed") void context.close();

    setTesting(false);
    setLevel(0);
  }

  const output = devices?.outputs.find((d) => d.isDefault);
  const input = devices?.inputs.find((d) => d.isDefault);

  return (
    <div className="h-full overflow-y-auto bg-transparent p-6 text-neutral-100">
      <header className="mb-7 flex items-start justify-between">
        <div>
          <p className="mb-1 text-xs uppercase tracking-[0.2em] text-neutral-500">
            System preferences
          </p>
          <h1 className="text-2xl font-semibold tracking-tight">Sound</h1>
          <p className="mt-1 text-sm text-neutral-400">
            Manage playback, input devices, and microphone privacy.
          </p>
        </div>
        <button
          onClick={() => void refresh()}
          disabled={busy}
          aria-label="Refresh audio devices"
          className="rounded-xl border border-white/10 bg-white/[0.04] p-2.5 text-neutral-300 transition hover:bg-white/[0.08] disabled:opacity-50"
        >
          <RefreshCw size={17} />
        </button>
      </header>

      {error && (
        <div className="mb-5 rounded-xl border border-red-400/20 bg-red-400/[0.07] p-3 text-sm text-red-200">
          {error}
        </div>
      )}

      <div className="grid gap-4 xl:grid-cols-2">
        <section className="rounded-2xl border border-white/[0.09] bg-white/[0.025] p-5">
          <div className="mb-5 flex items-center gap-3">
            <div className="rounded-xl bg-white/[0.07] p-3">
              <Speaker size={20} />
            </div>
            <div>
              <h2 className="font-medium">Output</h2>
              <p className="text-xs text-neutral-500">Speakers and headphones</p>
            </div>
            <div className="ml-auto text-neutral-500">
              <Volume2 size={19} />
            </div>
          </div>

          <label className="mb-2 block text-xs text-neutral-400">
            Output device
          </label>
          <select
            value={output?.name ?? ""}
            onChange={(e) => void selectOutput(e.target.value)}
            disabled={busy || !devices?.outputs.length}
            className="w-full appearance-none rounded-xl border border-white/10 bg-gradient-to-b from-white/[0.055] to-white/[0.025] px-3 py-2.5 pr-10 text-sm text-neutral-100 shadow-inner shadow-black/20 outline-none transition hover:border-white/20 focus:border-[var(--accent)]/70 focus:ring-2 focus:ring-[var(--accent)]/15 disabled:cursor-wait disabled:opacity-50 [color-scheme:dark]"
          >
            {!devices?.outputs.length && <option value="">No outputs found</option>}
            {devices?.outputs.map((device) => (
              <option key={device.name} value={device.name}>
                {device.description}{device.isDefault ? " · Default" : ""}
              </option>
            ))}
          </select>

          <div className="mt-4 flex items-center gap-3 rounded-xl bg-white/[0.035] p-3">
            <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-white/[0.06]">
              {output ? <Check size={16} /> : <VolumeX size={16} />}
            </div>
            <div className="min-w-0">
              <p className="text-sm">{output ? "Output ready" : "No output selected"}</p>
              <p className="truncate text-xs text-neutral-500">
                {output?.description ?? "Connect an audio output device"}
              </p>
            </div>
          </div>
        </section>

        <section className="rounded-2xl border border-white/[0.09] bg-white/[0.025] p-5">
          <div className="mb-5 flex items-center gap-3">
            <div className="rounded-xl bg-white/[0.07] p-3">
              <Mic size={20} />
            </div>
            <div>
              <h2 className="font-medium">Input</h2>
              <p className="text-xs text-neutral-500">Microphones and recording devices</p>
            </div>
            <button
              onClick={() => void toggleMicMute()}
              disabled={busy || !input}
              title={micMuted ? "Unmute microphone" : "Mute microphone"}
              className={`ml-auto rounded-xl border p-2.5 transition ${micMuted
                  ? "border-red-400/30 bg-red-400/10 text-red-200"
                  : "border-white/10 bg-white/[0.04] text-neutral-300 hover:bg-white/[0.08]"
                }`}
            >
              {micMuted ? <MicOff size={18} /> : <Mic size={18} />}
            </button>
          </div>

          <label className="mb-2 block text-xs text-neutral-400">
            Input device
          </label>
          <select
            value={input?.name ?? ""}
            onChange={(e) => void selectInput(e.target.value)}
            disabled={busy || !devices?.inputs.length}
            className="w-full appearance-none rounded-xl border border-white/10 bg-gradient-to-b from-white/[0.055] to-white/[0.025] px-3 py-2.5 pr-10 text-sm text-neutral-100 shadow-inner shadow-black/20 outline-none transition hover:border-white/20 focus:border-[var(--accent)]/70 focus:ring-2 focus:ring-[var(--accent)]/15 disabled:cursor-wait disabled:opacity-50 [color-scheme:dark]"
          >
            {!devices?.inputs.length && <option value="">No microphones found</option>}
            {devices?.inputs.map((device) => (
              <option key={device.name} value={device.name}>
                {device.description}{device.isDefault ? " · Default" : ""}
              </option>
            ))}
          </select>

          <div className="mt-4 flex items-center justify-between rounded-xl bg-white/[0.035] p-3">
            <div>
              <p className="text-sm">{micMuted ? "Microphone muted" : "Microphone active"}</p>
              <p className="mt-0.5 text-xs text-neutral-500">
                {micMuted ? "System input is muted" : "Input is available to apps"}
              </p>
            </div>
            <span className={`h-2 w-2 rounded-full ${micMuted ? "bg-red-400" : "bg-neutral-400"}`} />
          </div>
        </section>

        <section className="rounded-2xl border border-white/[0.09] bg-white/[0.025] p-5 xl:col-span-2">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-center gap-3">
              <div className="rounded-xl bg-white/[0.07] p-3">
                <AudioLines size={20} />
              </div>
              <div>
                <h2 className="font-medium">Microphone test</h2>
                <p className="mt-1 text-xs text-neutral-500">
                  Speak to check the input level and record a short sample.
                </p>
              </div>
            </div>
            <button
              onClick={() => void startTest()}
              disabled={micMuted || !input}
              className="rounded-xl border border-white/10 bg-white/[0.08] px-4 py-2.5 text-sm transition hover:bg-white/[0.13] disabled:cursor-not-allowed disabled:opacity-40"
            >
              {testing ? "Stop test" : "Start test"}
            </button>
          </div>

          <div className="mt-6 rounded-xl border border-white/[0.07] bg-black/20 p-4">
            <div className="mb-3 flex items-center justify-between text-xs">
              <span className="text-neutral-400">Input level</span>
              <span className="tabular-nums text-neutral-300">
                {testing ? `${level}%` : "Ready"}
              </span>
            </div>
            <div className="flex h-3 gap-1 overflow-hidden rounded-full">
              {Array.from({ length: 32 }, (_, i) => (
                <div
                  key={i}
                  className={`flex-1 rounded-sm transition-colors ${testing && i < Math.round((level / 100) * 32)
                      ? "bg-neutral-200"
                      : "bg-white/[0.08]"
                    }`}
                />
              ))}
            </div>
            <p className="mt-3 text-xs text-neutral-500">
              {micMuted
                ? "Unmute the microphone to run a test."
                : testing
                  ? "Speak normally. Stop the test to save the sample for playback."
                  : "Your recording stays in this window and is not uploaded."}
            </p>
          </div>

          {recordingUrl && (
            <div className="mt-4 flex flex-wrap items-center gap-3 rounded-xl border border-white/[0.07] p-3">
              <Play size={16} className="text-neutral-300" />
              <span className="text-sm">Test recording</span>
              <audio controls src={recordingUrl} className="ml-auto h-9 max-w-full" />
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
