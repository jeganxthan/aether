
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type PointerEvent as ReactPointerEvent,
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
const MIN_SCALE = 0.025;
const MAX_SCALE = 0.38;

function dimensions(display: DisplayOutput) {
  const mode = display.currentMode;
  const scale = Math.max(display.scale || 1, 0.1);

  if (!mode) {
    return {
      width: 1280 / scale,
      height: 720 / scale,
    };
  }

  const rotated = [
    "90",
    "270",
    "flipped-90",
    "flipped-270",
  ].includes(display.transform);

  return {
    width: (rotated ? mode.height : mode.width) / scale,
    height: (rotated ? mode.width : mode.height) / scale,
  };
}

function directionForDrop(
  bounds: DOMRect,
  clientX: number,
  clientY: number,
): Direction {
  const x = (clientX - bounds.left) / Math.max(bounds.width, 1);
  const y = (clientY - bounds.top) / Math.max(bounds.height, 1);

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
  const dragHandleRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef<DragState | null>(null);

  const [canvasWidth, setCanvasWidth] = useState(640);
  const [dragging, setDragging] = useState<DragState | null>(null);
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null);

  const updateDragState = useCallback((next: DragState | null) => {
    draggingRef.current = next;
    setDragging(next);
  }, []);

  useEffect(() => {
    const node = canvasRef.current;
    if (!node) return;

    const observer = new ResizeObserver(([entry]) => {
      if (entry) {
        setCanvasWidth(Math.max(1, entry.contentRect.width));
      }
    });

    observer.observe(node);

    return () => observer.disconnect();
  }, []);

  const layout = useMemo(() => {
    const items = displays.map((display) => {
      const position =
        positions[display.name] ?? { x: display.x, y: display.y };

      return {
        display,
        position,
        size: dimensions(display),
      };
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

    const availableWidth = Math.max(
      1,
      canvasWidth - CANVAS_PADDING * 2,
    );

    const availableHeight = CANVAS_HEIGHT - CANVAS_PADDING * 2;

    // The scale is calculated from the committed layout. It stays
    // unchanged while dragging because positions are not updated
    // until the pointer is released.
    const scale = Math.max(
      MIN_SCALE,
      Math.min(
        availableWidth / worldWidth,
        availableHeight / worldHeight,
        MAX_SCALE,
      ),
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

  const findDropTarget = useCallback(
    (
      source: string,
      clientX: number,
      clientY: number,
    ): DropTarget | null => {
      const element = document
        .elementFromPoint(clientX, clientY)
        ?.closest<HTMLElement>("[data-monitor-name]");

      const targetName = element?.dataset.monitorName;

      if (!element || !targetName || targetName === source) {
        return null;
      }

      return {
        name: targetName,
        direction: directionForDrop(
          element.getBoundingClientRect(),
          clientX,
          clientY,
        ),
      };
    },
    [],
  );

  const startDragging = useCallback(
    (
      event: ReactPointerEvent<HTMLDivElement>,
      name: string,
    ) => {
      if (
        name === primaryName ||
        event.button !== 0 ||
        draggingRef.current
      ) {
        return;
      }

      const monitorElement = event.currentTarget.closest<HTMLElement>(
        "[data-monitor-name]",
      );

      if (!monitorElement) return;

      event.preventDefault();

      const bounds = monitorElement.getBoundingClientRect();

      const next: DragState = {
        name,
        pointerId: event.pointerId,
        clientX: event.clientX,
        clientY: event.clientY,
        offsetX: event.clientX - bounds.left,
        offsetY: event.clientY - bounds.top,
      };

      try {
        event.currentTarget.setPointerCapture(event.pointerId);
      } catch {
        // The global pointer listeners below also handle the drag.
      }

      dragHandleRef.current = event.currentTarget;
      updateDragState(next);
      setDropTarget(null);
    },
    [primaryName, updateDragState],
  );

  const moveMonitor = useCallback(
    (source: string, target: string, direction: Direction) => {
      if (source === target || source === primaryName) return;

      const targetDisplay = displays.find(
        (display) => display.name === target,
      );

      const sourceDisplay = displays.find(
        (display) => display.name === source,
      );

      if (!targetDisplay || !sourceDisplay) return;

      const targetPosition =
        target === primaryName
          ? { x: 0, y: 0 }
          : positions[target] ?? {
              x: targetDisplay.x,
              y: targetDisplay.y,
            };

      const targetSize = dimensions(targetDisplay);
      const sourceSize = dimensions(sourceDisplay);

      let x = targetPosition.x;
      let y = targetPosition.y;

      switch (direction) {
        case "left":
          x -= sourceSize.width;
          y += (targetSize.height - sourceSize.height) / 2;
          break;

        case "right":
          x += targetSize.width;
          y += (targetSize.height - sourceSize.height) / 2;
          break;

        case "top":
          y -= sourceSize.height;
          x += (targetSize.width - sourceSize.width) / 2;
          break;

        case "bottom":
          y += targetSize.height;
          x += (targetSize.width - sourceSize.width) / 2;
          break;
      }

      const nextPositions = {
        ...positions,
        [primaryName]: { x: 0, y: 0 },
        [source]: { x, y },
      };

      onLayoutChange(nextPositions);
    },
    [displays, onLayoutChange, positions, primaryName],
  );

  const finishDragging = useCallback(
    (clientX: number, clientY: number, pointerId: number) => {
      const current = draggingRef.current;

      if (!current || current.pointerId !== pointerId) return;

      const target = findDropTarget(
        current.name,
        clientX,
        clientY,
      );

      // Clear drag state first so no stale pointer events can
      // accidentally trigger another placement.
      updateDragState(null);
      setDropTarget(null);

      const handle = dragHandleRef.current;
      dragHandleRef.current = null;

      if (handle?.hasPointerCapture(pointerId)) {
        try {
          handle.releasePointerCapture(pointerId);
        } catch {
          // Capture may already have been released by the browser.
        }
      }

      if (target) {
        moveMonitor(current.name, target.name, target.direction);
      }
    },
    [findDropTarget, moveMonitor, updateDragState],
  );

  const cancelDragging = useCallback(
    (pointerId: number) => {
      const current = draggingRef.current;

      if (!current || current.pointerId !== pointerId) return;

      const handle = dragHandleRef.current;
      dragHandleRef.current = null;

      updateDragState(null);
      setDropTarget(null);

      if (handle?.hasPointerCapture(pointerId)) {
        try {
          handle.releasePointerCapture(pointerId);
        } catch {
          // Capture may already have been released.
        }
      }
    },
    [updateDragState],
  );

  // Listen on window so the drag can finish even when the pointer
  // leaves the small handle or the preview canvas.
  useEffect(() => {
    if (!dragging) return;

    function handlePointerMove(event: globalThis.PointerEvent) {
      const current = draggingRef.current;

      if (!current || current.pointerId !== event.pointerId) return;

      event.preventDefault();

      const next = {
        ...current,
        clientX: event.clientX,
        clientY: event.clientY,
      };

      draggingRef.current = next;
      setDragging(next);

      setDropTarget(
        findDropTarget(
          current.name,
          event.clientX,
          event.clientY,
        ),
      );
    }

    function handlePointerUp(event: globalThis.PointerEvent) {
      finishDragging(event.clientX, event.clientY, event.pointerId);
    }

    function handlePointerCancel(event: globalThis.PointerEvent) {
      cancelDragging(event.pointerId);
    }

    window.addEventListener("pointermove", handlePointerMove, {
      passive: false,
    });
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("pointercancel", handlePointerCancel);

    return () => {
      window.removeEventListener("pointermove", handlePointerMove);
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("pointercancel", handlePointerCancel);
    };
  }, [
    dragging?.pointerId,
    findDropTarget,
    finishDragging,
    cancelDragging,
  ]);

  function setPrimary(name: string) {
    if (name === primaryName) return;

    const anchorPosition = positions[name] ?? {
      x: displays.find((display) => display.name === name)?.x ?? 0,
      y: displays.find((display) => display.name === name)?.y ?? 0,
    };

    const rebased: Record<string, DisplayPosition> = {};

    for (const display of displays) {
      const position = positions[display.name] ?? {
        x: display.x,
        y: display.y,
      };

      rebased[display.name] = {
        x: position.x - anchorPosition.x,
        y: position.y - anchorPosition.y,
      };
    }

    rebased[name] = { x: 0, y: 0 };

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
          {displays.length}{" "}
          {displays.length === 1 ? "display" : "displays"}
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
                  style={{
                    ...style,
                    zIndex: isDragging ? 1 : 0,
                  }}
                  className={[
                    "absolute flex min-h-0 flex-col rounded-xl border p-2 text-left",
                    "transition-colors duration-150",
                    isSelected
                      ? "border-[var(--accent)] bg-white/[0.08]"
                      : "border-white/10 bg-[#1b1c1f] hover:border-white/25",
                    isDragging ? "opacity-35" : "",
                    isDropTarget
                      ? "border-[var(--accent)] shadow-[0_0_24px_#9b87f544]"
                      : "",
                  ].join(" ")}
                  aria-label={`${display.name}${
                    isPrimary
                      ? ", primary display"
                      : ", draggable display"
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <div
                      ref={
                        isDragging
                          ? dragHandleRef
                          : undefined
                      }
                      onPointerDown={(event) =>
                        startDragging(event, display.name)
                      }
                      className={[
                        "touch-none select-none",
                        isPrimary
                          ? "cursor-default text-neutral-700"
                          : "cursor-grab text-neutral-500 active:cursor-grabbing",
                      ].join(" ")}
                      aria-label={
                        isPrimary
                          ? "Primary display"
                          : `Drag ${display.name}`
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
                      size={Math.min(
                        32,
                        Math.max(18, style.height * 0.3),
                      )}
                      strokeWidth={1.3}
                      className={
                        isSelected
                          ? "text-neutral-100"
                          : "text-neutral-500"
                      }
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
                    }}
                  >
                    <Monitor
                      size={24}
                      className="text-[var(--accent)]"
                    />

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
            {" · "}
            Drop on the top, bottom, left, or right edge to choose placement.
          </p>
        </>
      )}
    </section>
  );
}
