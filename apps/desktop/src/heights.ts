// Row heights for the virtualized reader (see DECISIONS.md D-012). Every row has a size,
// estimated until it has been rendered and measured; offsets are prefix sums, recomputed
// lazily from the first changed row.

export class Heights {
  readonly count: number;
  private sizes: Float64Array;
  private offsets: Float64Array;
  /** `offsets[i]` is stale for every `i >= dirty`; `offsets[0]` is always 0. */
  private dirty = 1;

  constructor(count: number, estimate: (row: number) => number) {
    this.count = count;
    this.sizes = new Float64Array(count);
    this.offsets = new Float64Array(count + 1);
    this.reset(estimate);
  }

  /** Replaces every size with a fresh estimate (after a font, measure or view change). */
  reset(estimate: (row: number) => number) {
    for (let i = 0; i < this.count; i++) this.sizes[i] = estimate(i);
    this.dirty = 1;
  }

  size(row: number): number {
    return this.sizes[row] ?? 0;
  }

  /** Records a measured size and returns how much it changed. */
  set(row: number, size: number): number {
    const delta = size - (this.sizes[row] ?? size);
    if (delta !== 0) {
      this.sizes[row] = size;
      this.dirty = Math.min(this.dirty, row + 1);
    }
    return delta;
  }

  /** Top of `row`; `offset(count)` is the total height. */
  offset(row: number): number {
    const target = Math.min(Math.max(row, 0), this.count);
    if (target >= this.dirty) {
      let y = this.offsets[this.dirty - 1] ?? 0;
      for (let i = this.dirty - 1; i < this.count; i++) {
        y += this.sizes[i] ?? 0;
        this.offsets[i + 1] = y;
      }
      this.dirty = this.count + 1;
    }
    return this.offsets[target] ?? 0;
  }

  total(): number {
    return this.offset(this.count);
  }

  /** The row at height `y` (clamped to the first and last row). */
  rowAt(y: number): number {
    this.offset(this.count);
    let lo = 0;
    let hi = this.count - 1;
    while (lo < hi) {
      const mid = (lo + hi + 1) >> 1;
      if ((this.offsets[mid] ?? 0) <= y) lo = mid;
      else hi = mid - 1;
    }
    return Math.max(lo, 0);
  }
}
