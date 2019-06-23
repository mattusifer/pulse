import { async, ComponentFixture, TestBed } from '@angular/core/testing';

import { SystemMonitorComponent } from './system-monitor.component';

describe('SystemMonitorComponent', () => {
  let component: SystemMonitorComponent;
  let fixture: ComponentFixture<SystemMonitorComponent>;

  beforeEach(async(() => {
    TestBed.configureTestingModule({
      declarations: [ SystemMonitorComponent ]
    })
    .compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(SystemMonitorComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
