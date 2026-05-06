import { Component, signal, ChangeDetectionStrategy } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { CommonModule } from '@angular/common';
import { TuiRoot, TuiIcon } from '@taiga-ui/core';
import { TuiNavigation } from '@taiga-ui/layout';

/**
 * Root application shell with TuiNavigation sidebar.
 *
 * The sidebar provides top-level navigation for the Angular mini-app.
 * When running inside an iframe (embedded in the React shell), the
 * sidebar is automatically hidden to avoid a double-sidebar UX issue.
 * When running standalone (future), the full sidebar is visible.
 *
 * Nav items link to either internal Angular routes or external React
 * routes (for features not yet migrated to Angular).
 */
@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    RouterLink,
    RouterLinkActive,
    TuiRoot,
    TuiNavigation,    // Includes TuiAsideComponent, TuiAsideItemDirective,
                       // TuiNavComponent, TuiMainComponent, etc.
    TuiIcon,
  ],
  templateUrl: './app.html',
  styleUrl: './app.less',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AppComponent {
  title = 'Dispatcharr';

  // Sidebar expanded state — collapsed by default for compact view
  readonly expanded = signal(false);

  // Detect if we're running inside an iframe (React shell)
  // If so, hide the sidebar to avoid double-sidebar UX
  readonly isEmbedded: boolean;

  // Navigation items — internal Angular routes
  readonly internalRoutes = [
    { path: '/', label: 'Channel Manager', icon: '@tui.settings', exact: true },
  ];

  // Navigation items — external links back to the React app
  // These will become internal routes as features are migrated
  readonly externalRoutes = [
    { href: '/channels', label: 'Channels', icon: '@tui.list-ordered' },
    { href: '/sources', label: 'M3U & EPG', icon: '@tui.play' },
    { href: '/guide', label: 'TV Guide', icon: '@tui.layout-grid' },
    { href: '/settings', label: 'Settings', icon: '@tui.sliders-horizontal' },
  ];

  constructor() {
    // Detect if we're inside an iframe — if so, the React shell already
    // provides sidebar navigation, so we hide ours to avoid duplication.
    try {
      this.isEmbedded = window.self !== window.top;
    } catch {
      // Cross-origin iframes throw SecurityError on the comparison
      this.isEmbedded = true;
    }
  }

  toggleExpanded(): void {
    this.expanded.update(v => !v);
  }
}
