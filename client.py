from typing import Any

import paho.mqtt.client as mqtt
from paho.mqtt.enums import CallbackAPIVersion, MQTTProtocolVersion
from paho.mqtt.properties import Properties
from paho.mqtt.reasoncodes import ReasonCode
from paho.mqtt.subscribeoptions import SubscribeOptions


# The callback for when the client receives a CONNACK response from the server.
def on_connect(
    client: mqtt.Client,
    userdata: Any,
    flags: mqtt.ConnectFlags,
    reason_code: ReasonCode,
    properties: Properties | None,
):
    print(f"Connected with result code {reason_code}")
    # Subscribing in on_connect() means that if we lose the connection and
    # reconnect then subscriptions will be renewed.
    client.subscribe(
        "some/topic",
        options=SubscribeOptions(
            qos=0,
            noLocal=False,
            retainAsPublished=False,
            retainHandling=SubscribeOptions.RETAIN_SEND_IF_NEW_SUB,
        ),
    )

    client.publish("some/topic", "data")


# The callback for when a PUBLISH message is received from the server.
def on_message(client, userdata, msg):
    print(f"{msg.topic} {str(msg.payload)}")


def on_disconnect(
    client: mqtt.Client,
    userdata: Any,
    flags: mqtt.DisconnectFlags,
    reason_code: ReasonCode,
    properties: Properties | None,
):
    print(f"Disconnected with result code {reason_code}")


mqttc = mqtt.Client(
    CallbackAPIVersion.VERSION2,
    protocol=MQTTProtocolVersion.MQTTv5,
    client_id="1",
)
mqttc.username_pw_set(username="username", password="password")
mqttc.on_connect = on_connect
mqttc.on_message = on_message
mqttc.on_disconnect = on_disconnect

mqttc.connect("localhost", 1883, 120)

# Blocking call that processes network traffic, dispatches callbacks and
# handles reconnecting.
# Other loop*() functions are available that give a threaded interface and a
# manual interface.
mqttc.loop_forever()
