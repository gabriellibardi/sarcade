export type Player = {
	id: number;
	nickname: string;
	is_host: boolean;
	connected: boolean;
};

export type RoomStatus = 'preparing' | 'waiting' | 'running' | 'failed';

export type RoomSnapshot = {
	code: string;
	game_id: number;
	sinteract: boolean;
	is_preview: boolean;
	status: RoomStatus;
	min_players: number;
	max_players: number;
	players: Player[];
	can_start: boolean;
	can_join: boolean;
	join_error: string | null;
};

export type RoomServerMessage =
	| {
			type: 'joined';
			player_id: number;
			is_host: boolean;
			player_token: string;
			room: RoomSnapshot;
	  }
	| { type: 'connection_replaced' }
	| { type: 'room_state'; room: RoomSnapshot }
	| { type: 'room_started'; room: RoomSnapshot }
	| { type: 'host_changed'; host_player_id: number; room: RoomSnapshot }
	| { type: 'player_kicked'; player_id: number; room: RoomSnapshot }
	| { type: 'player_left'; player_id: number; room: RoomSnapshot }
	| { type: 'frame'; svg: string }
	| { type: 'error'; message: string };
