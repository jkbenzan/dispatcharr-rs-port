with open('svelte-frontend/src/routes/stream-checker/+page.svelte', 'a', encoding='utf-8') as f:
    f.write('''
	/* =================== TREE VIEW (Stream Checker Left Pane) =================== */
	.tree-content {
		padding: 12px;
		background: var(--surface-bright);
	}
	.tree-container {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.tree-group {
		background: var(--surface);
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
	}
	.tree-row {
		display: flex;
		align-items: center;
		padding: 8px 12px;
		gap: 8px;
		cursor: pointer;
		user-select: none;
		transition: background 0.1s;

		&:hover {
			background: rgba(255,255,255,0.02);
		}
	}
	.group-row {
		background: rgba(0,0,0,0.1);
		border-bottom: 1px solid var(--border);
		font-weight: 600;
	}
	.channel-row {
		border-bottom: 1px solid var(--border);
		padding-left: 24px;
	}
	.stream-row {
		padding-left: 48px;
		padding-top: 10px;
		padding-bottom: 10px;
		border-bottom: 1px solid var(--border);
		
		&:last-child {
			border-bottom: none;
		}

		&.selected {
			background: rgba(255,255,255,0.05);
		}
	}
	.expand-btn {
		background: none;
		border: none;
		color: var(--text-dim);
		padding: 2px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		border-radius: 4px;

		&:hover {
			background: rgba(255,255,255,0.1);
			color: var(--text-bright);
		}
	}
	.checkbox-wrapper {
		display: flex;
		align-items: center;
	}
	.row-number {
		color: var(--text-dim);
		font-size: 13px;
	}
	.row-name {
		color: var(--text-bright);
		font-size: 14px;
	}
	.stream-info {
		display: flex;
		flex-direction: column;
		gap: 4px;
		flex: 1;
		min-width: 0;
	}
	.stream-main {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 13px;
	}
	.stream-name {
		color: var(--text-bright);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.stream-provider {
		background: rgba(255,255,255,0.1);
		padding: 2px 6px;
		border-radius: 4px;
		font-size: 10px;
		color: var(--text-dim);
		white-space: nowrap;
	}
	.stream-stats {
		display: flex;
		align-items: center;
		gap: 6px;
	}
	.stat-text {
		font-size: 11px;
		color: var(--text-dim);
	}
	.loading-row, .empty-row {
		padding: 12px 24px;
		color: var(--text-dim);
		font-size: 13px;
		display: flex;
		align-items: center;
		gap: 8px;
		font-style: italic;
	}
	.load-more-btn {
		width: 100%;
		padding: 10px;
		background: rgba(0,0,0,0.2);
		border: none;
		color: var(--text-dim);
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 6px;
		font-size: 13px;
		transition: all 0.2s;

		&:hover:not(:disabled) {
			background: rgba(255,255,255,0.05);
			color: var(--text-bright);
		}

		&:disabled {
			opacity: 0.5;
			cursor: not-allowed;
		}
	}
</style>''')
