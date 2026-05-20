export async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: "include",
    headers: { "Content-Type": "application/json", ...(init?.headers || {}) },
    ...init
  });

  if (!response.ok) {
    const text = await response.text();
    try {
      const parsed = JSON.parse(text) as { message?: string };
      throw new Error(parsed.message || text || `${response.status} ${response.statusText}`);
    } catch (error) {
      if (error instanceof SyntaxError) {
        throw new Error(text || `${response.status} ${response.statusText}`);
      }
      throw error;
    }
  }

  if (response.status === 204) return undefined as T;
  return response.json() as Promise<T>;
}
