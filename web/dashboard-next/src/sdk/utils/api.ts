import { stopVirtualMachine } from "../../lib/api";

/** Maps HyperSDK “cancel job” to KubeVirt VM stop (running → request stop). */
export async function cancelJob(jobId: string): Promise<void> {
  const slash = jobId.indexOf("/");
  if (slash < 1) {
    throw new Error("Invalid VM id (expected namespace/name)");
  }
  const ns = jobId.slice(0, slash);
  const name = jobId.slice(slash + 1);
  await stopVirtualMachine(ns, name);
}
