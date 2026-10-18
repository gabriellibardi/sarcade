export type ReviewDetails = {
	id: number;
	revision_id: number;
	revision_number: number;
	title: string;
	description: string;
	source_code: string;
	status: string;
	min_players: number;
	max_players: number;
	author_id: number | null;
	author_name: string | null;
	is_featured: boolean;
	review_message: string | null;
	created_at: string;
	updated_at: string;
	submitted_at: string | null;
	reviewed_at: string | null;
	has_thumbnail: boolean;
	runtime_build_id: string;
	runtime: string;
	runtime_name: string;
	runtime_version: string;
	source_extension: string;
};

export type OwnedRevision = {
	revision_id: number;
	revision_number: number;
	title: string;
	description: string;
	status: 'draft' | 'pending' | 'approved' | 'rejected' | 'withdrawn';
	min_players: number;
	max_players: number;
	review_message: string | null;
	created_at: string;
	updated_at: string;
	submitted_at: string | null;
	reviewed_at: string | null;
	has_thumbnail: boolean;
	runtime_build_id: string;
	runtime_name: string;
	runtime_version: string;
};

export type OwnedGame = {
	id: number;
	archived_at: string | null;
	revisions: OwnedRevision[];
};
