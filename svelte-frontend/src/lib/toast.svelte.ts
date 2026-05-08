export type ToastType = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
	id: number;
	message: string;
	type: ToastType;
	duration: number;
}

class ToastStore {
	private _toasts = $state<Toast[]>([]);
	private _nextId = 0;

	get toasts() {
		return this._toasts;
	}

	show(message: string, type: ToastType = 'info', duration: number = 3000) {
		const id = this._nextId++;
		const toast: Toast = { id, message, type, duration };
		this._toasts.push(toast);

		if (duration > 0) {
			setTimeout(() => {
				this.remove(id);
			}, duration);
		}
	}

	success(message: string, duration?: number) {
		this.show(message, 'success', duration);
	}

	error(message: string, duration?: number) {
		this.show(message, 'error', duration);
	}

	warning(message: string, duration?: number) {
		this.show(message, 'warning', duration);
	}

	info(message: string, duration?: number) {
		this.show(message, 'info', duration);
	}

	remove(id: number) {
		this._toasts = this._toasts.filter((t) => t.id !== id);
	}
}

export const toast = new ToastStore();
