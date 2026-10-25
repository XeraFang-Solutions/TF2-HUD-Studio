// Shared preview cache. An image name belongs to the current HUD and platform, never to the whole app session.
import { useEffect, useSyncExternalStore } from 'react';
import { api, type ImageResult } from '../api.ts';

const images = new Map<string, ImageResult | null>();
const pending = new Map<string, Promise<void>>();
const listeners = new Set<() => void>();
let generation = 0;
let revision = 0;
const subscribe = (listener: () => void) => { listeners.add(listener); return () => { listeners.delete(listener); }; };
const snapshot = () => revision;
const notify = () => { revision++; listeners.forEach((listener) => listener()); };

export function clearImageCache() {
  generation++;
  images.clear(); pending.clear(); notify();
}

function load(name: string, key: string) {
  if (images.has(key) || pending.has(key)) return;
  const started = generation;
  const request = api.image(name).then((image) => image, () => null).then((image) => {
    if (started !== generation) return; // A previous HUD's in-flight image must not repopulate the new cache.
    images.set(key, image); pending.delete(key); notify();
  });
  pending.set(key, request);
}

export function useCachedImage(name: string | undefined): ImageResult | null {
  const revision = useSyncExternalStore(subscribe, snapshot);
  const key = (name ?? '').replace(/\\/g, '/').toLowerCase();
  useEffect(() => { if (key) load(name!, key); }, [key, name, revision]);
  return images.get(key) ?? null;
}
