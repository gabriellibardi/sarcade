import { writable } from 'svelte/store';

export type AuthUser = {
	id: number;
	email: string;
	name: string;
	avatar: string | null;
	is_admin: boolean;
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

	pendingLoad = fetch('/api/auth/me')
		.then(async (response) => {
			const result = await response.json().catch(() => null);

			if (!response.ok || !result) {
				throw new Error('Não foi possível consultar a sessão.');
			}

			authState.set({
				user: result.user ?? null,
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
	const response = await fetch('/api/auth/logout', { method: 'POST' });

	if (!response.ok) {
		throw new Error('Não foi possível sair da conta.');
	}

	authState.update((state) => ({ ...state, user: null, error: '' }));
}
