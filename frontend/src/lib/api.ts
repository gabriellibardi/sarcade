export class ApiError extends Error {
	constructor(
		message: string,
		public readonly status: number
	) {
		super(message);
		this.name = 'ApiError';
	}
}

export async function apiRequest<T>(input: RequestInfo | URL, init?: RequestInit): Promise<T> {
	const response = await fetch(input, init);
	const text = await response.text();
	let payload: unknown = null;

	if (text) {
		try {
			payload = JSON.parse(text);
		} catch {
			payload = text;
		}
	}

	if (!response.ok) {
		throw new ApiError(
			responseMessage(payload) ?? 'A solicitação não pôde ser concluída.',
			response.status
		);
	}

	return payload as T;
}

export function errorMessage(error: unknown, fallback: string): string {
	return error instanceof ApiError ? error.message : fallback;
}

function responseMessage(payload: unknown): string | null {
	if (typeof payload === 'string' && payload.trim()) {
		return payload;
	}

	if (
		typeof payload === 'object' &&
		payload !== null &&
		'message' in payload &&
		typeof payload.message === 'string'
	) {
		return payload.message;
	}

	return null;
}
