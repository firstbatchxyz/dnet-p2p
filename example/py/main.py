from time import sleep
from socket import gethostname
from src.dnet_p2p import DnetP2P

def main():
    instancename = "my-app"
    hostname = gethostname()
    print(f"Using hostname: {hostname}")
    with DnetP2P("../../target/debug/libdnet_p2p.dylib") as dnet:
      dnet.enable_logs() # enables rust logging
      dnet.create_instance(instancename, hostname, is_manager=False)
      dnet.start()

      # sleep a bit
      print("Dnet service started. Press Ctrl+C to stop.")
      try:
          while True:
              sleep(1)
      except KeyboardInterrupt:   
          print("Stopping dnet service...")
    pass

if __name__ == "__main__":
    main()