export async function refundPayment(paymentId: string) {
  try {
    await stripe.refunds.create({ charge: paymentId })
  } catch (err) {
    if (err.status === 404) {
      return null
    }
    throw err
  }
}
