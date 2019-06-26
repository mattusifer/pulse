import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';

import { AppRoutingModule } from './app-routing.module';
import { AppComponent } from './app.component';
import { SystemMonitorComponent } from './system-monitor/system-monitor.component';
import { UiModule } from './ui/ui.module';

@NgModule({
    declarations: [
        AppComponent,
        SystemMonitorComponent,
        SystemMonitorComponent
    ],
    imports: [
        BrowserModule,
        AppRoutingModule,
        UiModule
    ],
    providers: [],
    bootstrap: [AppComponent]
})
export class AppModule { }
