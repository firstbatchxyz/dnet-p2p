from time import sleep
from src.dnet_p2p import DnetP2P
from secrets import token_hex
import sys


def main():
    is_manager = "-m" in sys.argv
    is_passive = "-p" in sys.argv
    instance = token_hex(12)

    if is_passive:
        print(f"Starting passive monitor {instance}")
    elif is_manager:
        print(f"Starting manager {instance}")
    else:
        print(f"Starting worker {instance}")

    with DnetP2P("../../lib") as dnet:
        dnet.enable_logs()  # enables rust logging
        dnet.create_instance(
            instance,
            8080,  # server_port
            50501,  # shard_port
            is_manager=is_manager,
            is_passive=is_passive,
        )
        dnet.start()

        print("Dnet service started. Press Ctrl+C to stop.")
        try:
            sleep(1)
            if not is_manager:
                # show own properties for shard
                properties = dnet.get_own_properties()
                fullname = dnet.fullname()
                print(fullname)
                print(properties.model_dump_json(indent=2))

            while True:
                if is_manager:
                    properties = dnet.get_properties(buffer_size=12000)
                    for service, properties in properties.items():
                        print(
                            f"Service: {service}:\n{properties.model_dump_json(indent=2)}"
                        )
                    print("=" * 80)
                sleep(5)
        except KeyboardInterrupt:
            print("Stopping dnet service...")
    pass


if __name__ == "__main__":
    main()
