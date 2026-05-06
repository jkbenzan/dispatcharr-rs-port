import { Routes } from '@angular/router';
import { ChannelManagerComponent } from './channel-manager/channel-manager.component';

/**
 * Application routes.
 *
 * The default route loads the Channel Manager. As more features are
 * migrated from the React frontend, additional routes will be added
 * here (e.g. /channels for the Channels Grid, /guide for TV Guide).
 */
export const routes: Routes = [
  // Channel Manager — the primary Angular-native feature
  { path: '', component: ChannelManagerComponent },

  // Catch-all: redirect unknown routes to Channel Manager
  { path: '**', redirectTo: '', pathMatch: 'full' },
];
