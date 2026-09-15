export function formatDate(
	value: string | null | undefined,
	options: Intl.DateTimeFormatOptions = { dateStyle: 'long' }
): string | null {
	if (!value) return null;

	const normalized = value.includes('T') ? value : `${value.replace(' ', 'T')}Z`;
	const date = new Date(normalized);

	return Number.isNaN(date.getTime())
		? null
		: new Intl.DateTimeFormat('pt-BR', options).format(date);
}

export function playerCountLabel(minPlayers: number, maxPlayers: number): string {
	if (minPlayers === maxPlayers) {
		return `${minPlayers} ${minPlayers === 1 ? 'jogador' : 'jogadores'}`;
	}

	return `${minPlayers}–${maxPlayers} jogadores`;
}
