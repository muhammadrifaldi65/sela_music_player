export type VolumeWriter = {
  readonly busy: boolean;
  readonly revision: number;
  set(value: number): void;
};
export function createVolumeWriter(
  write: (volume: number) => Promise<unknown>,
  onError: (error: unknown) => void,
): VolumeWriter {
  let pending: number | undefined,
    busy = false,
    revision = 0;
  async function flush() {
    busy = true;
    try {
      while (pending !== undefined) {
        const value = pending;
        pending = undefined;
        try {
          await write(value);
        } catch (error) {
          onError(error);
        }
      }
    } finally {
      busy = false;
    }
  }
  return {
    get busy() {
      return busy;
    },
    get revision() {
      return revision;
    },
    set(value: number) {
      pending = value;
      revision++;
      if (!busy) void flush();
    },
  };
}
