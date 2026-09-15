import type { GameSummary } from './game';

export type AuthUser = {
	id: number;
	email: string;
	name: string;
	avatar: string | null;
	is_admin: boolean;
};

export type PublicUserProfile = {
	id: number;
	name: string;
	avatar: string | null;
	created_at: string;
	published_games: GameSummary[];
};
