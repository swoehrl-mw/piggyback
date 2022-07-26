# Piggyback

Piggyback is a simple tool to do a reverse-port-forwarding for kubernetes. It allows you to expose a locally-running applications inside kubernetes. This is only intended for debugging and testing purposes and not for productive applications.

What piggyback does, is it deploys a simple proxy in kubernetes as a pod and creates a kubernetes service for it. Then it does a normal `kubectl port-forward` to that proxy and establishes a tunnel. Then whenever something connects to the proxy from inside kubernetes that connection is forwarded through the tunnel and sent to a local target service.

## Quickstart

You need:

* A kubernetes cluster and your local kubectl context pointing to it (try `kubectl cluster-info` to verify)
* The piggyback cli binary (download for your OS from the releases page)

Then in a first terminal start the application you want to expose (we use the included python http server for demonstration purposes):

```bash
python3 -m http.server 5000
```

In a second terminal start piggyback:

```bash
piggyback port-forward --deploy localhost:5000
# Wait until piggyback reports "Connected to proxy. Waiting for data"
```

Then in a third terminal simulate a connection from inside the cluster:

```bash
kubectl run --restart=Never --rm -i --image alpine/curl curl -- http://piggyback.default.svc.cluster.local:8080
```

If you then check your first terminal you will see that a request was received by the http server.

You can stop and start the `piggyback port-forward` command, the proxy pod in kubernetes will remain until you delete it with `piggyback delete`.

## Development

For development and quick local testing you can use k3d. The following steps deploy the proxy into your local k3d cluster:

* Build the proxy image: `docker build . -f Dockerfile-amd64local -t piggyback-proxy:dev`
* Push the image into k3d: `k3d image import piggyback-proxy:dev`
* Deploy the proxy: `cargo run --bin piggyback -- deploy --proxy-image piggyback-proxy:dev`
* Run a port-forward: `cargo run --bin piggyback -- port-forward localhost:5000`
