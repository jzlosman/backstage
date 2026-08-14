export interface SettledBatchEntry<Item, Result> {
  item: Item;
  result: PromiseSettledResult<Result>;
}

export async function settleBatches<Item, Result>(
  items: readonly Item[],
  batchSize: number,
  operation: (item: Item) => Promise<Result>,
  publish: (entries: readonly SettledBatchEntry<Item, Result>[]) => void,
  isCurrent: () => boolean,
) {
  const size = Math.max(1, Math.floor(batchSize));
  for (let offset = 0; offset < items.length; offset += size) {
    if (!isCurrent()) return;
    const batch = items.slice(offset, offset + size);
    const results = await Promise.allSettled(batch.map(operation));
    if (!isCurrent()) return;
    publish(batch.map((item, index) => ({ item, result: results[index]! })));
  }
}
