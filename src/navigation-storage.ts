export type NavigationState = {
  favorites: string[];
  recent: string[];
};

const LIMIT = 12;

function key(workspaceId: string) {
  return `reqvault.navigation.${workspaceId}`;
}

export function loadNavigation(workspaceId: string, availablePaths: string[]): NavigationState {
  const available = new Set(availablePaths);
  try {
    const parsed = JSON.parse(window.localStorage.getItem(key(workspaceId)) ?? "{}") as Partial<NavigationState>;
    return {
      favorites: Array.isArray(parsed.favorites) ? parsed.favorites.filter((path) => available.has(path)).slice(0, LIMIT) : [],
      recent: Array.isArray(parsed.recent) ? parsed.recent.filter((path) => available.has(path)).slice(0, LIMIT) : [],
    };
  } catch {
    window.localStorage.removeItem(key(workspaceId));
    return { favorites: [], recent: [] };
  }
}

export function saveNavigation(workspaceId: string, state: NavigationState) {
  window.localStorage.setItem(key(workspaceId), JSON.stringify({
    favorites: [...new Set(state.favorites)].slice(0, LIMIT),
    recent: [...new Set(state.recent)].slice(0, LIMIT),
  }));
}

export function addRecent(paths: string[], path: string): string[] {
  return [path, ...paths.filter((item) => item !== path)].slice(0, LIMIT);
}
