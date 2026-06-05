const YOCTO_PER_NEAR = 1_000_000_000_000_000_000_000_000n;

export function formatNear(yocto: string | number | bigint | null | undefined): string {
    if (yocto === null || yocto === undefined) {
        return "0 NEAR";
    }

    const value = BigInt(yocto);
    const whole = value / YOCTO_PER_NEAR;
    const fraction = value % YOCTO_PER_NEAR;

    if (fraction === 0n) {
        return `${whole.toString()} NEAR`;
    }

    const fractionText = fraction
        .toString()
        .padStart(24, "0")
        .replace(/0+$/, "")
        .slice(0, 4);

    return `${whole.toString()}.${fractionText} NEAR`;
}

export function formatTimestamp(timestamp: number | null): string {
    if (!timestamp) {
        return "Not set";
    }

    const milliseconds = Math.floor(timestamp / 1_000_000);

    return new Date(milliseconds).toLocaleString();
}