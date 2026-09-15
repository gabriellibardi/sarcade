import { writable } from 'svelte/store';
import { apiRequest } from '$lib/api';
import type { AuthUser } from '$lib/types/user';

type AuthResponse = {
	user: AuthUser | null;
	google_oauth_enabled: boolean;
};

type AuthState = {
	user: AuthUser | null;
	googleOAuthEnabled: boolean;
	isLoading: boolean;
	error: string;
};

export const authState = writable<AuthState>({
	user: null,
	googleOAuthEnabled: false,
	isLoading: true,
	error: ''
});

let pendingLoad: Promise<void> | null = null;

export function loadAuth(force = false): Promise<void> {
	if (pendingLoad && !force) {
		return pendingLoad;
	}

	pendingLoad = apiRequest<AuthResponse>('/api/auth/me')
		.then((result) => {
			authState.set({
				user: result.user,
				googleOAuthEnabled: Boolean(result.google_oauth_enabled),
				isLoading: false,
				error: ''
			});
		})
		.catch((error) => {
			console.error(error);
			authState.set({
				user: null,
				googleOAuthEnabled: false,
				isLoading: false,
				error: 'Não foi possível verificar o login.'
			});
		})
		.finally(() => {
			pendingLoad = null;
		});

	return pendingLoad;
}

export async function logout(): Promise<void> {
	await apiRequest('/api/auth/logout', { method: 'POST' });

	authState.update((state) => ({ ...state, user: null, error: '' }));
}
