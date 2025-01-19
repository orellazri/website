+++
title = "Beginning My Smart Home Journey with Zigbee"
date = "2025-01-19"

[taxonomies]
tags=["smart-home"]
+++

In the last few weeks, I've been wanting to make my home smarter, in an efficient and secure way.

I wanted to add smart devices that won't have access to my local network or to the internet, so I won't have to worry about separating my network and messing things up security-wise.

That's where I stubmled upon Zigbee and got into this rabbit hole.

## What is Zigbee?

From Wikipedia:
> Zigbee is a ... specification for a suite of high-level communication protocols used to create personal area networks with small, low-power digital radios, such as for home automation, ... designed for small scale projects which need wireless connection. Hence, Zigbee is a low-power, low-data-rate, and close proximity (i.e., personal area) wireless ad hoc network.

What that means for me is that my Zigbee-powered smart devices can be on a network of their own, with no access to the internet (since they don't receive IP addresses).

## What do I need?

For this to work, I need:

- Zigbee-powered smart device (obviously)
- Zigbee coordinator
- Some computer/SoC to run my software on

For the smart device, I started with a smart plug. For the coordinator, I bought a Sonoff ZBDongle-E which is a USB dongle with an antenna. For the computer, I have a small NUC that's running some of my services that need to run locally. A Raspberry Pi sort of device will also do the trick.

As for the software, I chose to run [Zigbee2MQTT](https://www.zigbee2mqtt.io/) which is a bridge that uses my USB dongle to connect to my Zigbee devices. Then, it can be added to Home Assistant/Homebridge/etc.

## Getting to It

First of all, my specific USB dongle needed to be flashed with a newer firmware version to support the Ember protocol which is the non-deprecated one. To do that, I used [Silabs Firmware Flasher](https://darkxst.github.io/silabs-firmware-builder/) which can easily flash that firmware over the browser. The other alternative is to disassemble the dongle and do something a bit more tricky, which I haven't tried because the first way worked for me.

After doing that, I deployed a simple Docker Compose stack with both Zigbee2MQTT and an MQTT broker (I'm using [Eclipse Mosquitto](https://mosquitto.org/):

```yaml
services:
  mqtt:
    image: eclipse-mosquitto:2.0
    container_name: mqtt
    restart: unless-stopped
    command: mosquitto -c /mosquitto-no-auth.conf
    volumes:
      - /home/<...>/volumes/zigbee/mosquitto:/mosquitto
    ports:
      - 1883:1883

  zigbee2mqtt:
    image: koenkk/zigbee2mqtt
    container_name: zigbee2mqtt
    restart: unless-stopped
    volumes:
      - /home/<...>>/volumes/zigbee/zigbee2mqtt:/app/data
      - /run/udev:/run/udev:ro
    ports:
      - 8082:8080
    environment:
      - TZ=...
    devices:
      - /dev/serial/by-id/<...>:/dev/ttyUSB0
```

Note that you'll need to find your USB dongle's ID in `/dev/serial/by-id/...`

After doing that, navigate to Z2M's (short for Zigbee2MQTT) web UI and make sure everything works. You can then use it to scan for devices.

### Integrating with Apple Home

In my case, I preferred to use Apple HomeKit instead of Home Assistant, so I deployed Homebridge which is a very useful service that exposes many types of smart devices to Apple Home:

```yaml
homebridge:
image: homebridge/homebridge:latest
container_name: homebridge
restart: unless-stopped
network_mode: host
volumes:
  - /home/<...>/volumes/homebridge:/homebridge
```

I also used the [homebridge-z2m](https://z2m.dev/) plugin inside Homebridge to integrate Zigbee2MQTT with Homebridge.
