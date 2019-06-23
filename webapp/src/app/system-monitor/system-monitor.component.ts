import { Component, OnInit } from '@angular/core';

import { Message } from './model/message';
import { SocketService } from './services/socket.service';
import * as Plotly from 'plotly.js';

const WEBSOCKET_URL = 'ws://localhost:8088/ws';

@Component({
    selector: 'app-system-monitor',
    templateUrl: './system-monitor.component.html',
    styleUrls: ['./system-monitor.component.scss']
})
export class SystemMonitorComponent implements OnInit {
    mounts: string[] = [];
    messages: Map<string, Message[]> = new Map();
    plots: Plotly.ScatterLine;
    private socket;

    constructor() { }

    newPlot(mount: string) {
        let x = this.messages.get(mount).map((x) => x.recordedAt.toString());
        let y = this.messages.get(mount).map((x) => x.percentDiskUsed);
        let data: Plotly.ScatterData[] = [
            {
                x: x,
                y: y,
                type: 'scatter'
            }
        ];
        Plotly.newPlot('disk-usage-' + mount, data)
    }

    ngOnInit() {
        this.socket = new WebSocket(WEBSOCKET_URL);
        this.socket.onmessage = (data: MessageEvent) => {
            let message = new Message().deserialize(JSON.parse(data.data));

            if (this.messages.has(message.mount)) {
                let messageArray = this.messages.get(message.mount) || [];
                messageArray.push(message)
                this.messages.set(message.mount, messageArray)
            } else {
                this.messages.set(message.mount, [message]);
                this.mounts.push(message.mount);
            }

            this.newPlot(message.mount)
        }
    }
}
