import { ComponentFixture, TestBed } from '@angular/core/testing';

import { CreateChannelDialog } from './create-channel-dialog';

describe('CreateChannelDialog', () => {
  let component: CreateChannelDialog;
  let fixture: ComponentFixture<CreateChannelDialog>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CreateChannelDialog],
    }).compileComponents();

    fixture = TestBed.createComponent(CreateChannelDialog);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
