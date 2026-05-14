<script lang="ts">
	import Modal from '$lib/components/ui/Modal.svelte';
	import { Save, X } from 'lucide-svelte';

	let {
		show = $bindable(false),
		groupName = '',
		onSave
	} = $props<{
		show: boolean;
		groupName: string;
		onSave: (newName: string) => void | Promise<void>;
	}>();

	let editName = $state('');
	let saving = $state(false);
	let inputRef = $state<HTMLInputElement | null>(null);
	
	// Update editName when modal opens
	$effect(() => {
		if (show) {
			editName = groupName;
			// Small timeout to ensure DOM is ready before focusing
			setTimeout(() => {
				if (inputRef) {
					inputRef.focus();
					inputRef.select();
				}
			}, 50);
		}
	});

	async function handleSave(e: Event) {
		e.preventDefault();
		if (!editName.trim() || editName === groupName) {
			show = false;
			return;
		}
		
		saving = true;
		try {
			await onSave(editName.trim());
			show = false;
		} finally {
			saving = false;
		}
	}
</script>

<Modal bind:show title="Edit Channel Group" width="450px">
	<form class="edit-group-form" onsubmit={handleSave}>
		<div class="form-group">
			<label for="groupName">Group Name</label>
			<input 
				type="text" 
				id="groupName" 
				bind:this={inputRef}
				bind:value={editName} 
				required 
				autocomplete="off"
				placeholder="Enter channel group name"
				disabled={saving}
			/>
		</div>

		<div class="modal-actions">
			<button type="button" class="btn secondary" onclick={() => show = false} disabled={saving}>
				<X size={16} /> Cancel
			</button>
			<button type="submit" class="btn primary" disabled={saving || !editName.trim() || editName === groupName}>
				<Save size={16} /> {saving ? 'Saving...' : 'Save Changes'}
			</button>
		</div>
	</form>
</Modal>

<style lang="less">
	.edit-group-form {
		padding: 24px;
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 8px;

		label {
			font-size: 13px;
			font-weight: 500;
			color: var(--text-dim);
		}

		input {
			padding: 10px 12px;
			background: rgba(0, 0, 0, 0.2);
			border: 1px solid var(--border);
			border-radius: var(--radius);
			color: var(--text-bright);
			font-size: 14px;
			transition: all 0.2s;

			&:focus {
				outline: none;
				border-color: var(--accent);
				background: rgba(0, 0, 0, 0.3);
			}

			&:disabled {
				opacity: 0.5;
				cursor: not-allowed;
			}
		}
	}

	.modal-actions {
		display: flex;
		justify-content: flex-end;
		gap: 12px;
		margin-top: 8px;
	}

	.btn {
		display: flex;
		align-items: center;
		gap: 8px;
		padding: 8px 16px;
		border-radius: var(--radius);
		font-size: 14px;
		font-weight: 500;
		cursor: pointer;
		border: none;
		transition: all 0.2s;

		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}

		&.secondary {
			background: rgba(255, 255, 255, 0.05);
			color: var(--text-bright);
			&:hover:not(:disabled) { background: rgba(255, 255, 255, 0.1); }
		}

		&.primary {
			background: var(--accent);
			color: white;
			&:hover:not(:disabled) { background: var(--accent-hover); }
		}
	}
</style>
