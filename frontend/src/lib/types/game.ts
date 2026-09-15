export type GameSummary = {
	id: number;
	revision_id: number;
	title: string;
	description: string;
	status: string;
	min_players: number;
	max_players: number;
	created_at: string;
	has_thumbnail: boolean;
	author_id: number | null;
	author_name: string | null;
	is_featured: boolean;
	runtime_build_id: string;
	runtime: string;
	runtime_name: string;
	runtime_version: string;
	source_extension: string;
};

export type PublishedGameDetails = {
	id: number;
	revision_id: number;
	title: string;
	description: string;
	min_players: number;
	max_players: number;
	author_id: number | null;
	author_name: string | null;
	is_featured: boolean;
	created_at: string;
	published_at: string | null;
	has_thumbnail: boolean;
	runtime_build_id: string;
	runtime: string;
	runtime_name: string;
	runtime_version: string;
	source_extension: string;
};

export type PublicUserProfile = {
	id: number;
	name: string;
	avatar: string | null;
	created_at: string;
	published_games: GameSummary[];
};
