const BUTTONDOWN_USER = 'fnlog0'
const BUTTONDOWN_EMBED_URL = `https://buttondown.com/api/emails/embed-subscribe/${BUTTONDOWN_USER}`

export function GetCTA() {
  return (
    <section id="get" className="vp-get">
      <h2 className="vp-get-title">Get early access</h2>
      <p className="vp-get-desc">
        Join the waitlist. Terminal + editor. Self-hosted LLMs. Your data, your control.
      </p>
      <form
        action={BUTTONDOWN_EMBED_URL}
        method="post"
        target="_blank"
        rel="noopener noreferrer"
        className="vp-get-form"
      >
        <label htmlFor="bd-email" className="vp-get-label">
          <span className="vp-get-label-text">Email</span>
          <input
            id="bd-email"
            type="email"
            name="email"
            placeholder="you@example.com"
            required
            className="vp-get-input"
          />
        </label>
        <button type="submit" className="button">
          Subscribe
        </button>
      </form>
      <p className="vp-get-hint">
        Powered by{' '}
        <a href={`https://buttondown.com/${BUTTONDOWN_USER}`} target="_blank" rel="noopener noreferrer">
          Buttondown
        </a>
      </p>
    </section>
  )
}
