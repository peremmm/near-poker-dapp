import type { Buffer as BufferType } from "buffer";
import type process from "process";

declare global {
    var Buffer: typeof BufferType;
    var global: typeof globalThis;
    var process: typeof process;
}

export {};