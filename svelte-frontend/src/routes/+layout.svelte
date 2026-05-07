<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { connectWS } from '$lib/ws.svelte';
	import { Home, Tv, Library, Settings, Activity, Search } from 'lucide-svelte';

	let { children } = $props();

	onMount(() => {
		connectWS();
	});

	const navItems = [
		{ name: 'Dashboard', icon: Home, href: '/' },
		{ name: 'Channels', icon: Tv, href: '/channels' },
		{ name: 'Streams', icon: Library, href: '/streams' },
		{ name: 'Activity', icon: Activity, href: '/activity' },
		{ name: 'Settings', icon: Settings, href: '/settings' }
	];
</script>

<div class="app-shell">
	<aside class="sidebar">
		<div class="logo">
			<span class="logo-text">DISPATCH<span class="accent">ARR</span></span>
		</div>
		
		<nav class="nav">
			{#each navItems as item}
				<a href={item.href} class="nav-item">
					<item.icon size={20} />
					<span>{item.name}</span>
				</a>
			{/each}
		</nav>

		<div class="sidebar-footer">
			<div class="user-profile">
				<div class="avatar">JB</div>
				<div class="user-info">
					<div class="username">Admin</div>
					<div class="status">Connected</div>
				</div>
			</div>
		</div>
	</aside>

	<main class="main-container">
		<header class="header">
			<div class="search-bar">
				<Search size={18} />
				<input type="text" placeholder="Search channels, movies, shows..." />
			</div>
			
			<div class="header-actions">
				<!-- Header icons go here -->
			</div>
		</header>

		<div class="content">
			{@render children()}
		</div>
	</main>
</div>

<style lang="less">
	.app-shell {
		display: flex;
		height: 100vh;
		width: 100vw;
		background: var(--bg);
	}

	.sidebar {
		width: var(--sidebar-width);
		background: var(--surface);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		padding: 24px 0;
	}

	.logo {
		padding: 0 24px 32px;
		.logo-text {
			font-size: 20px;
			font-weight: 800;
			letter-spacing: 1px;
			color: var(--text-bright);
			.accent { color: var(--accent); }
		}
	}

	.nav {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 0 12px;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 12px;
		color: var(--text-dim);
		text-decoration: none;
		font-size: 14px;
		font-weight: 500;
		border-radius: var(--radius);
		transition: all 0.2s;

		&:hover {
			background: rgba(255, 255, 255, 0.05);
			color: var(--text-bright);
		}

		&.active {
			background: rgba(237, 28, 36, 0.1);
			color: var(--accent);
		}
	}

	.sidebar-footer {
		padding: 24px;
		border-top: 1px solid var(--border);
	}

	.user-profile {
		display: flex;
		align-items: center;
		gap: 12px;
	}

	.avatar {
		width: 36px;
		height: 36px;
		background: var(--accent);
		color: white;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
		font-size: 12px;
	}

	.username {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-bright);
	}

	.status {
		font-size: 11px;
		color: #4ade80;
	}

	.main-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.header {
		height: var(--header-height);
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0 32px;
		border-bottom: 1px solid var(--border);
		background: var(--bg);
	}

	.search-bar {
		display: flex;
		align-items: center;
		gap: 12px;
		background: var(--surface-bright);
		padding: 8px 16px;
		border-radius: 20px;
		width: 400px;
		color: var(--text-dim);

		input {
			background: transparent;
			border: none;
			color: var(--text-bright);
			font-size: 14px;
			width: 100%;
			&:focus { outline: none; }
		}
	}

	.content {
		flex: 1;
		overflow-y: auto;
		padding: 32px;
	}
</style>
