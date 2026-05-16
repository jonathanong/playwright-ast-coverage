// Good: all exports start with "process" and none end with "Job"
export async function processWelcomeEmail(job: any) {
  await Promise.resolve(job.data);
}

export const processBounceNotification = async (job: any) => {
  await Promise.resolve(job.data);
};

export { processWelcomeEmail as processWelcomeEmailAlias };
