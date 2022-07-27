use k8s_openapi::api::core::v1::{Pod, Service};
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams, PostParams},
    runtime::wait::{
        await_condition,
        conditions::{is_deleted, is_pod_running},
    },
    Client, Config, ResourceExt,
};
use tokio::io::{AsyncRead, AsyncWrite};

async fn apis(namespace: Option<String>) -> (Api<Pod>, Api<Service>, String) {
    let namespace = match namespace {
        Some(namespace) => namespace,
        None => {
            let config = Config::infer().await.unwrap();
            config.default_namespace
        }
    };
    let client = Client::try_default().await.unwrap();
    let pod_api: Api<Pod> = Api::namespaced(client.clone(), &namespace);
    let service_api: Api<Service> = Api::namespaced(client, &namespace);
    (pod_api, service_api, namespace)
}

pub async fn deploy_proxy(name: &str, namespace: Option<String>, port: u32) {
    let (pod_api, service_api, namespace) = apis(namespace).await;

    // create pod
    let pod: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": {
                "app.kubernetes.io/instance": name,
                "app.kubernetes.io/name": "piggyback"
            }
        },
        "spec": {
            "containers": [{
                "name": "piggyback",
                "image": format!("ghcr.io/swoehrl-mw/piggyback-proxy:{}", env!("GIT_TAG")),
                "imagePullPolicy": "Always",
                "env": [
                    {
                        "name": "PROXY_PORT",
                        "value": format!("{}", port),
                    }
                ],
                "ports": [
                    {
                        "name": "http",
                        "containerPort": port,
                        "protocol": "TCP"
                    },
                    {
                        "name": "control",
                        "containerPort": 12345,
                        "protocol": "TCP"
                    }
                ]
            }],
        }
    }))
    .unwrap();

    let create = if let Ok(existing_pod) = pod_api.get(name).await {
        if !are_pods_equal(&existing_pod, &pod) {
            println!("Deleting existing proxy pod");
            pod_api
                .delete(name, &DeleteParams::default())
                .await
                .unwrap();
            let uid = existing_pod.uid().unwrap();
            let running = await_condition(pod_api.clone(), name, is_deleted(uid.as_str()));
            let _ = tokio::time::timeout(std::time::Duration::from_secs(30), running)
                .await
                .expect("Failed to delete pod");
            true
        } else {
            false
        }
    } else {
        true
    };
    if create {
        println!("Creating proxy pod");
        pod_api.create(&PostParams::default(), &pod).await.unwrap();
        let running = await_condition(pod_api.clone(), name, is_pod_running());
        let _ = tokio::time::timeout(std::time::Duration::from_secs(30), running)
            .await
            .expect("Proxy pod did not start");
    }

    // create service
    let service: Service = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": {
                "app.kubernetes.io/instance": name,
                "app.kubernetes.io/name": "piggyback"
            }
        },
        "spec": {
            "ports": [
                {
                    "name": "http",
                    "port": port,
                    "protocol": "TCP",
                    "targetPort": "http"
                },
                {
                    "name": "control",
                    "port": 12345,
                    "protocol": "TCP",
                    "targetPort": "control"
                }
            ],
            "selector": {
                "app.kubernetes.io/instance": name,
                "app.kubernetes.io/name": "piggyback"
            },
            "type": "ClusterIP"
        },
    }))
    .unwrap();

    println!("Creating/updating service");
    service_api
        .patch(
            name,
            &PatchParams::apply("piggyback"),
            &Patch::Apply(service),
        )
        .await
        .expect("Failed to apply service");
    println!("Proxy deployed");
}

fn are_pods_equal(existing_pod: &Pod, new_pod: &Pod) -> bool {
    let existing_container = existing_pod
        .spec
        .as_ref()
        .unwrap()
        .containers
        .first()
        .unwrap();
    let new_container = new_pod.spec.as_ref().unwrap().containers.first().unwrap();
    if existing_container.image != new_container.image {
        return false;
    }
    if existing_container
        .ports
        .as_ref()
        .unwrap()
        .first()
        .unwrap()
        .container_port
        != new_container
            .ports
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .container_port
    {
        return false;
    }
    true
}

pub async fn delete_proxy(name: &str, namespace: Option<String>) {
    let (pod_api, service_api, _) = apis(namespace).await;
    println!("Deleting pod and service");
    let _ = pod_api.delete(name, &DeleteParams::default()).await;
    let _ = service_api.delete(name, &DeleteParams::default()).await;
}

pub async fn portforward(
    name: &str,
    namespace: Option<String>,
) -> impl AsyncRead + AsyncWrite + Unpin {
    let (pod_api, _, _) = apis(namespace).await;

    let mut res = pod_api.portforward(name, &[12345]).await.unwrap();

    res.take_stream(12345).unwrap()
}
