interface MiniCavaProps {
    playing: boolean;
}

const bars = [1, 2, 3];

export default function MiniCava({ playing }: MiniCavaProps) {
    return (
        <div className="mini-cava">
            {bars.map((bar) => (
                <span
                    key={bar}
                    className={`mini-cava-bar bar-${bar}`}
                    style={{
                        animationPlayState: playing
                            ? "running"
                            : "paused",
                    }}
                />
            ))}
        </div>
    );
}
