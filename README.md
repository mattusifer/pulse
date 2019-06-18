pulse
------------
A monitor and job scheduler

[![CircleCI](https://circleci.com/gh/mattusifer/pulse.svg?style=svg)](https://circleci.com/gh/mattusifer/pulse)

### Usage
Clone, build, and run

```bash
$ ./target/release/pulse >> pulse.log 2>&1 &
```

### Configuration
Configured via ~/.pulse/config.toml

#### Example
```toml
###
### Configure the scheduler
###

# Run the scheduler every 5 seconds
[scheduler]
tick_ms = 5000

# Send a news digest at 9am every day
[[scheduler.schedules]]
cron = "0 0 9 * * * *"
message = "fetch-news"

# Check disk usage
#   If no cron key is specified in a [[scheduler.schedules]]
#   block, the scheduler will perform this operation on every tick
[[scheduler.schedules]]
message = "check-disk-usage"

###
### Configure operations
###

# Configure check-disk-usage operation
#   Send an alert if the filesytem mounted at '/' has more
#   than 90% disk usage
[[system_monitor.filesystems]]
mount = "/"
available_space_alert_above = 90.0

# Configure fetch-news operation
#   Add a connection to the NYT API and configure which sections
#   to include in the digest
[news.new_york_times]
api_key = "nyt-api-key"
most_popular_viewed_period = "7"
most_popular_emailed_period = "7"
most_popular_shared_period = "7"
most_popular_shared_mediums = ["facebook"]

###
### Configure alerts
###

# Configure email for alerts
[broadcast.email]
smtp_host = "smtp.gmail.com"
username = "user@gmail.com"
password = "password"
recipients = ["recipient1@gmail.com", "recipient2@gmail.com"]

# Configure the high-disk-usage alert
#   Only send the high-disk-usage alert once every hour
[[broadcast.alerts]]
alert_interval = { secs = 3600, nanos = 0 }
mediums = ["email"]
event = "high-disk-usage"
alert_type = "alarm"

# Configure the news digest alert
[[broadcast.alerts]]
mediums = ["email"]
event = "newscast"
alert_type = "digest"
```
