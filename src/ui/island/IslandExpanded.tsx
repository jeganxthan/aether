export default function IslandExpanded() {
  return (
    <div className="px-5 pb-5">
      <div className="border-t border-neutral-800 pt-4">
        <div className="grid grid-cols-3 gap-2">

          {/* Network */}
          <div className="rounded-xl border border-neutral-900 bg-neutral-950 p-3">
            <p className="text-[9px] tracking-widest text-neutral-600">
              NETWORK
            </p>

            <p className="mt-1 text-xs text-neutral-300">
              Connected
            </p>
          </div>

          {/* Battery */}
          <div className="rounded-xl border border-neutral-900 bg-neutral-950 p-3">
            <p className="text-[9px] tracking-widest text-neutral-600">
              BATTERY
            </p>

            <p className="mt-1 text-xs text-neutral-300">
              87%
            </p>
          </div>

          {/* Audio */}
          <div className="rounded-xl border border-neutral-900 bg-neutral-950 p-3">
            <p className="text-[9px] tracking-widest text-neutral-600">
              AUDIO
            </p>

            <p className="mt-1 text-xs text-neutral-300">
              42%
            </p>
          </div>

        </div>
      </div>
    </div>
  );
}