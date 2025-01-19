+++
title = "Deploying Ingress Nginx on EKS With an Internal Load Balancer and HTTPS"
date = "2024-03-31"

[taxonomies]
tags=["devops"]
+++

In this post, we will deploy Ingress Nginx on EKS with an internal load balancer and HTTPS. This setup is useful when you want to expose your services an internal network (e.g. your work VPN) and secure the communication with HTTPS.

The TLS termination will be done by the NLB, so the traffic between the client and the load balancer will be encrypted.

## Prerequisites

- An EKS cluster
- `kubectl` installed
- SSL certificates in AWS ACM

## Deploy Ingress Nginx

First, we need to deploy Ingress Nginx. We will use the deployment files (I couldn't get the Helm chart to work properly since it was assigning the wrong target groups to the NLB).

Download the deploy.yaml file from the [official repository](https://raw.githubusercontent.com/kubernetes/ingress-nginx/controller-v1.10.0/deploy/static/provider/aws/nlb-with-tls-termination/deploy.yaml)

Open the file, and modify the `proxy-real-ip-cidr: XXX.XXX.XXX/XX` field to match the VPC CIDR that your EKS cluster is using (e.g. 10.67.0.0/22).

You will also need to change `arn:aws:acm:us-west-2:XXXXXXXX:certificate/XXXXXX-XXXXXXX-XXXXXXX-XXXXXXXX` to match the ARN of your SSL certificate in AWS ACM.

Then, deploy the manifest:

```bash
kubectl apply -f deploy.yaml
```

This should create the Ingress Nginx controller and the necessary resources on K8s, along with an internal NLB on AWS.

## Create an Ingress

To test the setup, create an Ingress resource that points to a service in your cluster.

```yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: my-ingress
spec:
  ingressClassName: nginx
  rules:
    - host: my-host.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: my-service
                port:
                  number: 80
```

Replace the `host` field with the domain you want to use, and the `service` field with the name of the service you want to expose (along with the correct port number).
Note that the `ingressClassName` field is set to `nginx`.

Then, apply the manifest:

```bash
kubectl apply -f ingress.yaml
```
