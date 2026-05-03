import { Component, inject, OnInit } from '@angular/core';
import { ApiService } from '../api.service';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-channel-manager',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './channel-manager.component.html',
  styleUrl: './channel-manager.component.less'
})
export class ChannelManagerComponent implements OnInit {
  private api = inject(ApiService);
  
  groups: any[] = [];

  ngOnInit() {
    this.api.getChannelGroups().subscribe(
      (res: any) => {
        this.groups = Array.isArray(res) ? res : res?.results || [];
      },
      err => console.error('Error fetching groups', err)
    );
  }
}
