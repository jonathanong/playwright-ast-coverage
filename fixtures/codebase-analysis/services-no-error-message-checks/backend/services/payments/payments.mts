export async function processPayment(paymentId: string) {
  try {
    await stripe.charges.create({ id: paymentId })
  } catch (err) {
    if (err.message === 'Not Found') {
      return null
    }
    throw err
  }
}
