import { BrowserModule } from '@angular/platform-browser';
import { NgModule } from '@angular/core';

import { AppRoutingModule } from './app-routing.module';
import { AppComponent } from './app.component';
import { SystemMonitorComponent } from './system-monitor/system-monitor.component';

@NgModule({
    declarations: [
        AppComponent,
        SystemMonitorComponent,
        SystemMonitorComponent
    ],
    imports: [
        BrowserModule,
        AppRoutingModule
    ],
    providers: [],
    bootstrap: [AppComponent]
})
export class AppModule { }
