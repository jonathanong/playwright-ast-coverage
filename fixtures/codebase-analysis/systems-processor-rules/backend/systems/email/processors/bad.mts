// Bad: "handleEmail" doesn't start with "process"; "sendEmailJob" ends with "Job"
export async function handleEmail(job: any) {
  await Promise.resolve(job.data);
}

export const sendEmailJob = async (job: any) => {
  await Promise.resolve(job.data);
};
