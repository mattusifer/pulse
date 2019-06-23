interface Serializable<T> {
    deserialize(input: Object): T;
}

export class Message implements Serializable<Message> {
    public mount: string;
    public percentDiskUsed: number;
    public recordedAt: Date;

    deserialize(input) {
        this.mount = input.mount;
        this.percentDiskUsed = input.percent_disk_used;
        this.recordedAt = input.recorded_at;

        return this;
    }
}
