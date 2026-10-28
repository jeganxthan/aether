import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent,
} from "react";
import {
  ArrowDown,
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Grip,
  Monitor,
  Star,
} from "lucide-react";
import type { DisplayOutput } from "../../services/display";

export interface DisplayPosition {
  x: number;
  y: number;
}

type Direction = "left" | "right" | "top" | "bottom";

interface DragState {
  name: string;
  pointerId: number;
  clientX: number;
  clientY: number;
  offsetX: number;
  offsetY: number;
}

interface DropTarget {
  name: string;
  direction: Direction;
}

interface MonitorLayoutProps {
  displays: DisplayOutput[];
  positions: Record<string, DisplayPosition>;
  selectedName: string;
  primaryName: string;
  onSelect: (name: string) => void;
  onPrimaryChange: (name: string) => void;
  onLayoutChange: (positions: Record<string, DisplayPosition>) => void;
}

const CANVAS_HEIGHT = 280;
const CANVAS_PADDING = 24;

function dimensions(display: DisplayOutput) {
  const mode = display.currentMode;
  if (!mode) return { width: 1280 / display.scale, height: 720 / display.scale };

  const rotated = ["90", "270", "flipped-90", "flipped-270"].includes(
    display.transform,
  );
  return {
    width: (rotated ? mode.height : mode.width) / display.scale,
    height: (rotated ? mode.width : mode.height) / display.scale,
  };
}

function directionForDrop(
  bounds: DOMRect,
  clientX: number,
  clientY: number,
): Direction {
  const x = (clientX - bounds.left) / bounds.width;
  const y = (clientY - bounds.top) / bounds.height;
  const horizontalDistance = Math.abs(x - 0.5);
  const verticalDistance = Math.abs(y - 0.5);

  if (horizontalDistance >= verticalDistance) {
    return x < 0.5 ? "left" : "right";
  }
  return y < 0.5 ? "top" : "bottom";
}

function placementIcon(direction: Direction) {
  switch (direction) {
    case "left":
      return <ArrowLeft size={18} />;
    case "right":
      return <ArrowRight size={18} />;
    case "top":
      return <ArrowUp size={18} />;
    case "bottom":
      return <ArrowDown size={18} />;
  }
}

function placementPosition(direction: Direction) {
  switch (direction) {
    case "left":
      return "left-0 top-0 h-full w-9";
    case "right":
      return "right-0 top-0 h-full w-9";
    case "top":
      return "left-0 top-0 h-9 w-full";
    case "bottom":
      return "bottom-0 left-0 h-9 w-full";
  }
}

export default function MonitorLayout({
  displays,
  positions,
  selectedName,
  primaryName,
  onSelect,
  onPrimaryChange,
  onLayoutChange,
}: MonitorLayoutProps) {
  const canvasRef = useRef<HTMLDivElement>(null);
  const [canvasWidth, setCanvasWidth] = useState(640);
  const [dragging, setDragging] = useState<DragState | null>(null);
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null);

  useEffect(() => {
    const node = canvasRef.current;
    if (!node) return;

    const observer = new ResizeObserver(([entry]) => {
      setCanvasWidth(entry.contentRect.width);
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  const layout = useMemo(() => {
    const items = displays.map((display) => {
      const position = positions[display.name] ?? { x: display.x, y: display.y };
      return { display, position, size: dimensions(display) };
    });
    if (items.length === 0) return [];

    const minX = Math.min(...items.map((item) => item.position.x));
    const minY = Math.min(...items.map((item) => item.position.y));
    const maxX = Math.max(
      ...items.map((item) => item.position.x + item.size.width),
    );
    const maxY = Math.max(
      ...items.map((item) => item.position.y + item.size.height),
    );
    const worldWidth = Math.max(maxX - minX, 1);
    const worldHeight = Math.max(maxY - minY, 1);
    const scale = Math.min(
      (canvasWidth - CANVAS_PADDING * 2) / worldWidth,
      (CANVAS_HEIGHT - CANVAS_PADDING * 2) / worldHeight,
      0.38,
    );
    const scaledWidth = worldWidth * scale;
    const scaledHeight = worldHeight * scale;
    const offsetX = (canvasWidth - scaledWidth) / 2;
    const offsetY = (CANVAS_HEIGHT - scaledHeight) / 2;

    return items.map(({ display, position, size }) => ({
      display,
      style: {
        left: offsetX + (position.x - minX) * scale,
        top: offsetY + (position.y - minY) * scale,
        width: Math.max(72, size.width * scale),
        height: Math.max(50, size.height * scale),
      },
    }));
  }, [canvasWidth, displays, positions]);

  function findDropTarget(
    source: string,
    clientX: number,
    clientY: number,
  ): DropTarget | null {
    const element = document
      .elementFromPoint(clientX, clientY)
      ?.closest<HTMLElement>("[data-monitor-name]");
    const targetName = element?.dataset.monitorName;
    if (!element || !targetName || targetName === source) return null;

    return {
      name: targetName,
      direction: directionForDrop(
        element.getBoundingClientRect(),
        clientX,
        clientY,
      ),
    };
  }

  function startDragging(
    event: PointerEvent<HTMLDivElement>,
    name: string,
  ) {
    if (name === primaryName || event.button !== 0) return;

    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    const bounds = event.currentTarget
      .closest<HTMLElement>("[data-monitor-name]")
      ?.getBoundingClientRect();
    if (!bounds) return;

    setDragging({
      name,
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
      offsetX: event.clientX - bounds.left,
      offsetY: event.clientY - bounds.top,
    });
  }

  function updateDragging(event: PointerEvent<HTMLDivElement>) {
    if (!dragging || dragging.pointerId !== event.pointerId) return;

    setDragging((current) =>
      current
        ? {
            ...current,
            clientX: event.clientX,
            clientY: event.clientY,
          }
        : current,
    );
    setDropTarget(
      findDropTarget(dragging.name, event.clientX, event.clientY),
    );
  }

  function finishDragging(event: PointerEvent<HTMLDivElement>) {
    if (!dragging || dragging.pointerId !== event.pointerId) return;

    const target = findDropTarget(
      dragging.name,
      event.clientX,
      event.clientY,
    );
    if (target) {
      moveMonitor(dragging.name, target.name, target.direction);
    }

    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setDragging(null);
    setDropTarget(null);
  }

  function cancelDragging(event: PointerEvent<HTMLDivElement>) {
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    setDragging(null);
    setDropTarget(null);
  }

  function moveMonitor(source: string, target: string, direction: Direction) {
    if (source === target || source === primaryName) return;

    const targetDisplay = displays.find((display) => display.name === target);
    const sourceDisplay = displays.find((display) => display.name === source);
    if (!targetDisplay || !sourceDisplay) return;

    const targetPosition = positions[target] ?? {
      x: targetDisplay.x,
      y: targetDisplay.y,
    };
    const targetSize = dimensions(targetDisplay);
    const sourceSize = dimensions(sourceDisplay);
    const nextPosition: DisplayPosition = {
      x: targetPosition.x,
      y: targetPosition.y,
    };

    if (direction === "left") nextPosition.x -= sourceSize.width;
    if (direction === "right") nextPosition.x += targetSize.width;
    if (direction === "top") nextPosition.y -= sourceSize.height;
    if (direction === "bottom") nextPosition.y += targetSize.height;

    onLayoutChange({ ...positions, [source]: nextPosition });
  }

  function setPrimary(name: string) {
    const anchorDisplay = displays.find((display) => display.name === name);
    const anchorPosition = positions[name] ?? {
      x: anchorDisplay?.x ?? 0,
      y: anchorDisplay?.y ?? 0,
    };
    const rebased = Object.fromEntries(
      displays.map((display) => {
        const position = positions[display.name] ?? {
          x: display.x,
          y: display.y,
        };
        return [
          display.name,
          {
            x: position.x - anchorPosition.x,
            y: position.y - anchorPosition.y,
          },
        ];
      }),
    );

    onPrimaryChange(name);
    onSelect(name);
    onLayoutChange(rebased);
  }

  return (
    <section className="rounded-2xl border border-white/[0.08] bg-white/[0.02] p-5">
      <div className="mb-5 flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-medium text-neutral-100">
            Monitor arrangement
          </h3>
          <p className="mt-1 text-xs text-neutral-500">
            Grab a monitor by its handle and drag it to an edge to place it.
            Change the primary display to move the anchored monitor.
          </p>
        </div>
        <span className="rounded-full border border-white/10 px-2.5 py-1 text-[11px] text-neutral-400">
          {displays.length} {displays.length === 1 ? "display" : "displays"}
        </span>
      </div>

      {displays.length === 0 ? (
        <p className="py-8 text-center text-sm text-neutral-500">
          No active monitors detected.
        </p>
      ) : (
        <>
          <div
            ref={canvasRef}
            className="relative h-[280px] overflow-hidden rounded-xl border border-dashed border-white/10 bg-black/10"
            aria-label="Draggable monitor layout"
          >
            {layout.map(({ display, style }) => {
              const isSelected = display.name === selectedName;
              const isPrimary = display.name === primaryName;
              const isDragging = display.name === dragging?.name;
              const isDropTarget = dropTarget?.name === display.name;

              return (
                <div
                  key={display.name}
                  data-monitor-name={display.name}
                  style={style}
                  className={[
                    "absolute flex min-h-0 flex-col rounded-xl border p-2 text-left transition-[left,top,width,height,opacity,transform,border-color] duration-200",
                    isSelected
                      ? "border-[var(--accent)] bg-white/[0.08]"
                      : "border-white/10 bg-[#1b1c1f] hover:border-white/25",
                    "cursor-default",
                    isDragging ? "scale-95 opacity-40" : "",
                    isDropTarget
                      ? "border-[var(--accent)] shadow-[0_0_24px_#9b87f544]"
                      : "",
                  ].join(" ")}
                  aria-label={`${display.name}${isPrimary ? ", primary display" : ", draggable display"}`}
                >
                  <div className="flex items-center justify-between">
                    <div
                      onPointerDown={(event) =>
                        startDragging(event, display.name)
                      }
                      onPointerMove={updateDragging}
                      onPointerUp={finishDragging}
                      onPointerCancel={cancelDragging}
                      className={
                        isPrimary
                          ? "cursor-default text-neutral-700"
                          : "touch-none cursor-grab text-neutral-500 active:cursor-grabbing"
                      }
                      aria-label={
                        isPrimary ? "Primary display" : `Drag ${display.name}`
                      }
                      title={
                        isPrimary
                          ? "Primary display"
                          : "Drag this display to another monitor"
                      }
                    >
                      <Grip size={13} />
                    </div>
                    {isPrimary && (
                      <span className="flex items-center gap-1 rounded-full bg-[var(--accent)]/15 px-1.5 py-0.5 text-[9px] text-neutral-200">
                        <Star size={9} />
                        Primary
                      </span>
                    )}
                  </div>
                  <button
                    type="button"
                    onClick={() => onSelect(display.name)}
                    className="flex min-h-0 flex-1 flex-col items-center justify-center gap-1 overflow-hidden text-center"
                  >
                    <Monitor
                      size={Math.min(32, Math.max(18, style.height * 0.3))}
                      strokeWidth={1.3}
                      className={isSelected ? "text-neutral-100" : "text-neutral-500"}
                    />
                    <span className="max-w-full truncate text-[10px] font-medium text-neutral-100">
                      {display.name}
                    </span>
                    <span className="text-[9px] text-neutral-500">
                      {display.currentMode
                        ? `${display.currentMode.width} × ${display.currentMode.height}`
                        : "Resolution unavailable"}
                    </span>
                  </button>
                  {isDropTarget && dropTarget && (
                    <span
                      className={[
                        "pointer-events-none absolute flex items-center justify-center rounded-xl bg-[var(--accent)]/20 text-[var(--accent)]",
                        placementPosition(dropTarget.direction),
                      ].join(" ")}
                    >
                      {placementIcon(dropTarget.direction)}
                    </span>
                  )}
                  {!isPrimary && (
                    <button
                      type="button"
                      onClick={() => setPrimary(display.name)}
                      className="mt-1 w-full truncate rounded-md border border-white/10 px-1 py-1 text-[9px] text-neutral-300 transition hover:bg-white/[0.08]"
                    >
                      Set primary
                    </button>
                  )}
                </div>
              );
            })}
            {dragging &&
              (() => {
                const source = layout.find(
                  ({ display }) => display.name === dragging.name,
                );
                if (!source) return null;
                return (
                  <div
                    className="pointer-events-none fixed z-50 flex flex-col items-center justify-center rounded-xl border border-[var(--accent)] bg-[#202027]/95 p-2 text-center shadow-[0_12px_36px_#0009] backdrop-blur-md"
                    style={{
                      left: dragging.clientX - dragging.offsetX,
                      top: dragging.clientY - dragging.offsetY,
                      width: source.style.width,
                      height: source.style.height,
                      transform: "scale(1.03)",
                    }}
                  >
                    <Monitor size={24} className="text-[var(--accent)]" />
                    <span className="mt-1 max-w-full truncate text-[10px] font-medium text-neutral-100">
                      {source.display.name}
                    </span>
                  </div>
                );
              })()}
          </div>

          <p className="mt-3 text-xs text-neutral-500">
            Primary:{" "}
            <span className="text-neutral-300">
              {primaryName || "Not selected"}
            </span>
            {" · "}Drop on the top, bottom, left, or right edge to choose placement.
          </p>
        </>
      )}
    </section>
  );
}
