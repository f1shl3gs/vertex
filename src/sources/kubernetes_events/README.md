## Example
Here is an example deployment of the collector that sets up this source.

### Configuration
```yaml
apiVersion: v1
kind: ServiceAccount
metadata:
  labels:
    app: vertex
  name: vertex
---
kind: ClusterRole
apiVersion: rbac.authorization.k8s.io/v1
metadata:
  name: vertex
  labels:
    app: vertex
rules:
  - apiGroups:
      - ""
    resources:
      - events
      - namespaces
      - namespaces/status
      - nodes
      - nodes/spec
      - pods
      - pods/status
      - replicationcontrollers
      - replicationcontrollers/status
      - resourcequotas
      - services
    verbs:
      - get
      - list
      - watch
  - apiGroups:
      - apps
    resources:
      - daemonsets
      - deployments
      - replicasets
      - statefulsets
    verbs:
      - get
      - list
      - watch
  - apiGroups:
      - extensions
    resources:
      - daemonsets
      - deployments
      - replicasets
    verbs:
      - get
      - list
      - watch
  - apiGroups:
      - batch
    resources:
      - jobs
      - cronjobs
    verbs:
      - get
      - list
      - watch
  - apiGroups:
      - autoscaling
    resources:
      - horizontalpodautoscalers
    verbs:
      - get
      - list
      - watch
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: vertex
  labels:
    app: vertex
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: vertex
subjects:
  - kind: ServiceAccount
    name: vertex
    namespace: default
---
kind: ConfigMap
apiVersion: v1
metadata:
  name: vertex
data:
  config.yaml: |
    sources:
      events:
        type: kubernetes_events
        namespaces: []

    sinks:
      stdout:
        type: stdout
        inputs:
          - events
---
kind: Deployment
apiVersion: apps/v1
metadata:
  name: vertex
  labels:
    app: vertex
spec:
  replicas: 1
  selector:
    matchLabels:
      app: vertex
  template:
    metadata:
      labels:
        app: vertex
    spec:
      serviceAccountName: vertex
      containers:
        - name: vertex
          image: f1shl3gs/vertex:nightly-distroless
          imagePullPolicy: IfNotPresent
          volumeMounts:
            - mountPath: /etc/vertex
              name: config
              readOnly: true
      volumes:
        - name: config
          configMap:
            name: vertex
            items:
              - key: config.yaml
                path: vertex.yaml
```

`Note`: the image this example used is nightly, 
and it's just a demonstration, replace to stable 
version when you deploy it in production.