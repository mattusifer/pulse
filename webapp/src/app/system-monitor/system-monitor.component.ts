import { Component, OnInit } from '@angular/core';

import { Message } from './model/message';
import { Chart, ChartPoint } from "chart.js";
import * as moment from "moment";

const WEBSOCKET_URL = 'ws://localhost:8088/ws';

const TICK_MS = 200;
const DATAPOINTS = 20;

@Component({
    selector: 'app-system-monitor',
    templateUrl: './system-monitor.component.html',
    styleUrls: ['./system-monitor.component.scss']
})
export class SystemMonitorComponent implements OnInit {
    mounts: Set<string> = new Set();
    messages: Map<string, Message[]> = new Map();

    private socket: WebSocket;
    private charts: Map<string, Chart> = new Map();

    constructor() { }

    adjustPlot(mount: string, message: Message) {
        if (!this.charts.has(mount)) {
            let canvas = <HTMLCanvasElement>document.getElementById(
                "disk-usage-" + mount
            );
            if (canvas === null) return;

            let ctx = canvas.getContext("2d");
            let chart = new Chart(ctx, {
                type: "line",
                data: {
                    datasets: [
                        {
                            label: "Disk Usage",
                            backgroundColor: "#000000",
                            data: []
                        }
                    ]
                },
                options: {
                    scales: {
                        xAxes: [{
                            type: 'time',
                            time: {
                                min: moment().utcOffset(-5).valueOf().toString(),
                                unit: 'millisecond',
                                unitStepSize: 400,
                            }
                        }],
                        yAxes: [{
                            ticks: {
                                min: 0,
                                max: 100
                            }
                        }]
                    },
                    maintainAspectRatio: false
                }
            });
            chart.update({ duration: 0, lazy: false, easing: 'linear' });
            this.charts.set(mount, chart);
        } else {
            let chart = this.charts.get(mount);
            let dataset = chart.data.datasets[0];
            if (dataset.data.length > DATAPOINTS) {
                dataset.data = dataset.data.slice(-1 * DATAPOINTS);
            }
            dataset.data.push({
                t: message.recordedAt, y: message.percentDiskUsed
            });

            chart.config.options.scales.xAxes =
                chart.config.options.scales.xAxes.map(a => {
                    a.time.min = moment(parseInt(a.time.min))
                        .utcOffset(-5)
                        .add(TICK_MS, 'milliseconds')
                        .valueOf()
                        .toString();
                    return a
                });
            chart.update({ duration: 0, lazy: false, easing: 'linear' });
        }
    }

    ngOnInit() {
        this.socket = new WebSocket(WEBSOCKET_URL);
        this.socket.onmessage = (data: MessageEvent) => {
            let message = new Message().deserialize(
                JSON.parse(data.data)
            );
            this.mounts.add(message.mount);

            this.adjustPlot(message.mount, message);
        }
    }
}
