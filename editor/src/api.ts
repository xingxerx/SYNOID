import type { Asset, TranscribeResponse, SessionResponse, RenderStatus } from './types';

const BASE = '/api/editor';

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...options?.headers },
    ...options,
  });
  if (!res.ok) {
    const text = await res.text().catch(() => 'Unknown error');
    throw new Error(`API ${res.status}: ${text}`);
  }
  return res.json();
}

// ─── Sessions ──────────────────────────────────────────────────────────────
export async function createSession(): Promise<SessionResponse> {
  return request('/sessions', { method: 'POST' });
}

export async function getSession(id: string): Promise<SessionResponse> {
  return request(`/sessions/${id}`);
}

// ─── Assets ────────────────────────────────────────────────────────────────
export async function uploadAsset(sessionId: string, file: File): Promise<Asset> {
  const form = new FormData();
  form.append('file', file);
  const res = await fetch(`${BASE}/sessions/${sessionId}/assets`, {
    method: 'POST',
    body: form,
  });
  if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
  return res.json();
}

export async function listAssets(sessionId: string): Promise<Asset[]> {
  return request(`/sessions/${sessionId}/assets`);
}

export async function deleteAsset(sessionId: string, assetId: string): Promise<void> {
  await fetch(`${BASE}/sessions/${sessionId}/assets/${assetId}`, { method: 'DELETE' });
}

export function assetStreamUrl(sessionId: string, assetId: string): string {
  return `${BASE}/sessions/${sessionId}/assets/${assetId}/stream`;
}

export function assetThumbnailUrl(sessionId: string, assetId: string): string {
  return `${BASE}/sessions/${sessionId}/assets/${assetId}/thumbnail`;
}

// ─── Transcription ─────────────────────────────────────────────────────────
export async function transcribeAsset(sessionId: string, assetId: string): Promise<TranscribeResponse> {
  return request(`/sessions/${sessionId}/transcribe`, {
    method: 'POST',
    body: JSON.stringify({ assetId }),
  });
}

// ─── Project ───────────────────────────────────────────────────────────────
export async function saveProject(sessionId: string, data: unknown): Promise<void> {
  await request(`/sessions/${sessionId}/project/save`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function loadProject(sessionId: string): Promise<unknown> {
  return request(`/sessions/${sessionId}/project/load`);
}

// ─── AI ────────────────────────────────────────────────────────────────────
export async function aiChat(sessionId: string, message: string): Promise<{ response: string; actions?: { type: string; label: string; params: Record<string, unknown> }[] }> {
  return request(`/sessions/${sessionId}/ai/chat`, {
    method: 'POST',
    body: JSON.stringify({ message }),
  });
}

export async function aiAutoEdit(sessionId: string, action: string, params: Record<string, unknown>): Promise<unknown> {
  return request(`/sessions/${sessionId}/ai/auto-edit`, {
    method: 'POST',
    body: JSON.stringify({ action, ...params }),
  });
}

// ─── Render ────────────────────────────────────────────────────────────────
export async function startRender(sessionId: string, projectData: unknown): Promise<{ jobId: string }> {
  return request(`/sessions/${sessionId}/render`, {
    method: 'POST',
    body: JSON.stringify(projectData),
  });
}

export async function getRenderStatus(sessionId: string): Promise<RenderStatus> {
  return request(`/sessions/${sessionId}/render/status`);
}
