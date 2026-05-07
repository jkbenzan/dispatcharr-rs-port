<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { connectWS } from '$lib/ws.svelte';
	import { Home, Tv, Library, Settings, Activity, Search, Sun, Moon, Palette, HardDrive, Link, Puzzle, HeartPulse, CalendarDays, Film } from 'lucide-svelte';

	let { children } = $props();

	onMount(() => {
		connectWS();
	});

	const navItems = [
		{ name: 'Dashboard', icon: Home, href: '/' },
		{ name: 'Channels', icon: Tv, href: '/channels' },
		{ name: 'Streams', icon: Library, href: '/streams' },
		{ name: 'TV Guide', icon: CalendarDays, href: '/guide' },
		{ name: 'VOD', icon: Film, href: '/vod' },
		{ name: 'DVR', icon: HardDrive, href: '/dvr' },
		{ name: 'Stream Checker', icon: HeartPulse, href: '/stream-checker' },
		{ name: 'Integrations', icon: Link, href: '/integrations' },
		{ name: 'Plugins', icon: Puzzle, href: '/plugins' },
		{ name: 'Activity', icon: Activity, href: '/activity' },
		{ name: 'Settings', icon: Settings, href: '/settings' }
	];

	// Theming & Layout
	let theme = $state('dark');
	let accent = $state('red');
	let sidebarCollapsed = $state(false);

	onMount(() => {
		const storedTheme = localStorage.getItem('theme');
		const storedAccent = localStorage.getItem('accent');
		const storedCollapsed = localStorage.getItem('sidebarCollapsed');
		if (storedTheme) theme = storedTheme;
		if (storedAccent) accent = storedAccent;
		if (storedCollapsed === 'true') sidebarCollapsed = true;
	});

	$effect(() => {
		if (typeof document !== 'undefined') {
			document.documentElement.setAttribute('data-theme', theme);
			document.documentElement.setAttribute('data-accent', accent);
			localStorage.setItem('theme', theme);
			localStorage.setItem('accent', accent);
			localStorage.setItem('sidebarCollapsed', sidebarCollapsed.toString());
		}
	});

	function toggleTheme() {
		theme = theme === 'dark' ? 'light' : 'dark';
	}

	function toggleAccent() {
		accent = accent === 'red' ? 'green' : 'red';
	}

	function toggleSidebar() {
		sidebarCollapsed = !sidebarCollapsed;
	}
</script>

<div class="app-shell" class:collapsed={sidebarCollapsed}>
	<aside class="sidebar">
		<div class="logo">
			<button class="toggle-btn" onclick={toggleSidebar} aria-label="Toggle Sidebar">
				<div class="logo-full">
					<span class="logo-text">DISPATCH<span class="accent">ARR</span></span>
					<span class="rust-flare">🚀 Powered by Rust</span>
				</div>
				<svg class="minimal-logo" viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
					<path d="M25 15 L85 50 L25 85 Z" fill="none" stroke="var(--text-bright)" stroke-width="12" stroke-linejoin="round" />
					<polygon points="40,35 40,65 65,50" fill="var(--accent)" />
					<path d="M 20 90 Q 50 100 80 80" fill="none" stroke="var(--accent)" stroke-width="10" stroke-linecap="round" />
				</svg>
			</button>
		</div>
		
		<nav class="nav">
			{#each navItems as item}
				<a href={item.href} class="nav-item" title={sidebarCollapsed ? item.name : undefined}>
					<item.icon size={20} />
					<span>{item.name}</span>
				</a>
			{/each}
		</nav>

		<div class="sidebar-footer">
			<div class="user-profile" title={sidebarCollapsed ? "Admin - Connected" : undefined}>
				<div class="avatar">JB</div>
				<div class="user-info">
					<div class="username">Admin</div>
					<div class="status">Connected</div>
				</div>
			</div>
			
			<div class="theme-controls">
				<button class="theme-btn" onclick={toggleTheme} title="Toggle Light/Dark Mode">
					{#if theme === 'dark'}
						<Sun size={16} />
					{:else}
						<Moon size={16} />
					{/if}
				</button>
				<button class="theme-btn" onclick={toggleAccent} title="Toggle Accent Color">
					<Palette size={16} color={accent === 'red' ? '#ed1c24' : '#158f76'} />
				</button>
			</div>
		</div>
	</aside>

	<main class="main-container">
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
		background: var(--glass-bg);
		backdrop-filter: var(--glass);
		-webkit-backdrop-filter: var(--glass);
		border-right: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		padding: 24px 0;
		transition: width 0.2s ease;
		box-shadow: 1px 0 30px rgba(0, 0, 0, 0.2);
		z-index: 100;
	}

	.app-shell.collapsed .sidebar {
		width: 72px;
	}

	.logo {
		padding: 0 24px 32px;
		display: flex;
		flex-direction: column;

		.toggle-btn {
			background: transparent;
			border: none;
			text-align: left;
			cursor: pointer;
			display: flex;
			flex-direction: column;
			padding: 0;
		}

		.logo-full {
			display: flex;
			flex-direction: column;
		}

		.logo-text {
			font-size: 20px;
			font-weight: 800;
			letter-spacing: 1px;
			color: var(--text-bright);
			.accent { color: var(--accent); }
		}

		.rust-flare {
			font-size: 8px;
			color: #f7a41d;
			font-weight: 700;
			letter-spacing: 0.5px;
			text-transform: uppercase;
			margin-top: 2px;
			white-space: nowrap;
		}

		.minimal-logo {
			display: none;
			width: 32px;
			height: 32px;
		}
	}

	.app-shell.collapsed .logo {
		padding: 0 0 32px 0;
		align-items: center;

		.logo-full {
			display: none;
		}

		.minimal-logo {
			display: block;
			margin: 0 auto;
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
		white-space: nowrap;
		overflow: hidden;

		&:hover {
			background: rgba(255, 255, 255, 0.05);
			color: var(--text-bright);
		}

		&.active {
			background: rgba(237, 28, 36, 0.1);
			color: var(--accent);
		}
	}

	.app-shell.collapsed .nav-item {
		justify-content: center;
		padding: 10px;
		
		span {
			display: none;
		}
	}

	.sidebar-footer {
		padding: 24px;
		border-top: 1px solid var(--border);
		display: flex;
		flex-direction: column;
		gap: 16px;
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
		flex-shrink: 0;
	}

	.username {
		font-size: 14px;
		font-weight: 600;
		color: var(--text-bright);
		white-space: nowrap;
	}

	.status {
		font-size: 11px;
		color: #4ade80;
	}

	.theme-controls {
		display: flex;
		gap: 8px;
	}

	.theme-btn {
		background: var(--surface-bright);
		border: 1px solid var(--border);
		color: var(--text-dim);
		width: 32px;
		height: 32px;
		border-radius: var(--radius);
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: all 0.2s;
		flex-shrink: 0;

		&:hover {
			color: var(--text-bright);
			border-color: var(--text-dim);
		}
	}

	.app-shell.collapsed .sidebar-footer {
		padding: 24px 0;
		align-items: center;
	}

	.app-shell.collapsed .user-info {
		display: none;
	}

	.app-shell.collapsed .theme-controls {
		flex-direction: column;
	}

	.color-dot {
		width: 12px;
		height: 12px;
		border-radius: 50%;
	}

	.main-container {
		flex: 1;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}


	.content {
		flex: 1;
		overflow-y: auto;
		padding: 32px;
	}
</style>
