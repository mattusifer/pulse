import { Component, OnInit } from '@angular/core';

import { Message } from './model/message';
import { Chart } from "chart.js";

const WEBSOCKET_URL = 'ws://localhost:8088/ws';

@Component({
    selector: 'app-system-monitor',
    templateUrl: './system-monitor.component.html',
    styleUrls: ['./system-monitor.component.scss']
})
export class SystemMonitorComponent implements OnInit {
    mounts: string[] = [];
    messages: Map<string, Message[]> = new Map();
    private socket;

    constructor() { }

    newPlot(mount: string) {
        let xs = this.messages.get(mount).map((x) => x.recordedAt);
        let ys = this.messages.get(mount).map((x) => x.percentDiskUsed);

        let canvas = <HTMLCanvasElement>document.getElementById("disk-usage-" + mount);
        let ctx = canvas.getContext("2d");
        new Chart(ctx, {
            type: "line",
            data: {
                datasets: [
                    {
                        label: "Disk Usage",
                        backgroundColor: "#000000",
                        data: xs.map((x, i) => { return { t: x, y: ys[i] } })
                    }
                ]
            },
            options: {
                scales: {
                    xAxes: [{
                        type: 'time'
                    }]
                }
            }
        }).update({ duration: 0, lazy: false, easing: 'linear' });
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
