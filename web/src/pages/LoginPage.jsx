import { useEffect, useMemo, useRef, useState } from 'react'
import axios from 'axios'
import './LoginPage.css'

function formatExpiry(expiresAt) {
    try {
        return new Date(expiresAt).toLocaleString()
    } catch (_error) {
        return expiresAt
    }
}

export default function LoginPage() {
    const telegramLoginRef = useRef(null)
    const [pageLoading, setPageLoading] = useState(true)
    const [verifyLoading, setVerifyLoading] = useState(false)
    const [pageError, setPageError] = useState(null)
    const [challenge, setChallenge] = useState(null)
    const [authResult, setAuthResult] = useState(null)
    const [copyStatus, setCopyStatus] = useState('复制授权码')

    const query = useMemo(() => new URLSearchParams(window.location.search), [])
    const clientId = query.get('client_id') || ''
    const state = query.get('state') || ''
    const source = query.get('source') || ''

    useEffect(() => {
        if (!clientId || !state) {
            setPageError('缺少 client_id 或 state，无法继续登录。')
            setPageLoading(false)
            return
        }

        if (source !== 'cli') {
            setPageError('当前页面仅支持 CLI 登录流程。')
            setPageLoading(false)
            return
        }

        let cancelled = false

        async function initChallenge() {
            try {
                const response = await axios.get('/api/cli/login/challenge', {
                    params: {
                        client_id: clientId,
                        state,
                        source
                    }
                })

                if (!cancelled) {
                    setChallenge(response.data)
                }
            } catch (error) {
                if (!cancelled) {
                    setPageError(error.response?.data?.error || '初始化登录流程失败')
                }
            } finally {
                if (!cancelled) {
                    setPageLoading(false)
                }
            }
        }

        initChallenge()

        return () => {
            cancelled = true
        }
    }, [clientId, source, state])

    useEffect(() => {
        if (pageLoading || pageError || authResult || !challenge || !telegramLoginRef.current) {
            return
        }

        let botUsername = import.meta.env.VITE_BOT_USERNAME
        if (!botUsername || /^\d+$/.test(botUsername)) {
            setPageError('请配置 VITE_BOT_USERNAME 为 Telegram bot 用户名。')
            return
        }

        botUsername = botUsername.replace('@', '')

        window.onTelegramCliAuth = async function onTelegramCliAuth(user) {
            setVerifyLoading(true)
            setPageError(null)

            try {
                const response = await axios.post('/api/cli/login/telegram/verify', {
                    client_id: clientId,
                    state,
                    telegram_login: {
                        id: user.id,
                        first_name: user.first_name,
                        last_name: user.last_name || null,
                        username: user.username || null,
                        photo_url: user.photo_url || null,
                        auth_date: user.auth_date,
                        hash: user.hash
                    }
                })
                setAuthResult(response.data)
            } catch (error) {
                setPageError(error.response?.data?.error || 'Telegram 登录验证失败')
            } finally {
                setVerifyLoading(false)
            }
        }

        telegramLoginRef.current.innerHTML = ''
        const script = document.createElement('script')
        script.async = true
        script.src = 'https://telegram.org/js/telegram-widget.js?22'
        script.setAttribute('data-telegram-login', botUsername)
        script.setAttribute('data-size', 'large')
        script.setAttribute('data-request-access', 'write')
        script.setAttribute('data-onauth', 'onTelegramCliAuth(user)')
        telegramLoginRef.current.appendChild(script)

        return () => {
            delete window.onTelegramCliAuth
        }
    }, [authResult, challenge, clientId, pageError, pageLoading, state])

    async function copyAuthorizationCode() {
        if (!authResult?.authorization_code) {
            return
        }

        try {
            await navigator.clipboard.writeText(authResult.authorization_code)
            setCopyStatus('已复制')
            window.setTimeout(() => setCopyStatus('复制授权码'), 1500)
        } catch (_error) {
            setCopyStatus('复制失败')
            window.setTimeout(() => setCopyStatus('复制授权码'), 1500)
        }
    }

    return (
        <div className="cli-login-page">
            <div className="cli-login-shell">
                <div className="cli-login-card">
                    <p className="cli-login-kicker">Nyamedia CLI Login</p>
                    <h1>使用 Telegram 完成 CLI 授权</h1>
                    <p className="cli-login-subtitle">
                        登录成功后，这个页面会生成一次性授权码。把它粘贴回 CLI 即可完成登录。
                    </p>

                    <div className="cli-login-meta">
                        <span>client_id: {clientId || '-'}</span>
                        <span>state: {state || '-'}</span>
                    </div>

                    {pageLoading && (
                        <div className="cli-login-panel">
                            <div className="cli-spinner" />
                            <p>正在初始化登录流程...</p>
                        </div>
                    )}

                    {!pageLoading && pageError && (
                        <div className="cli-login-panel cli-login-error">
                            <p>{pageError}</p>
                        </div>
                    )}

                    {!pageLoading && !pageError && !authResult && (
                        <div className="cli-login-panel">
                            <p className="cli-login-step">步骤 1：使用 Telegram 登录</p>
                            <div ref={telegramLoginRef} className="cli-widget-slot" />
                            {challenge?.expires_at && (
                                <p className="cli-login-hint">
                                    本次登录流程有效期至 {formatExpiry(challenge.expires_at)}
                                </p>
                            )}
                            {verifyLoading && <p className="cli-login-hint">正在验证 Telegram 登录...</p>}
                        </div>
                    )}

                    {!pageLoading && !pageError && authResult && (
                        <div className="cli-login-panel cli-login-success">
                            <p className="cli-login-step">步骤 2：复制一次性授权码</p>
                            <div className="cli-code-block">{authResult.authorization_code}</div>
                            <button className="cli-copy-button" onClick={copyAuthorizationCode}>
                                {copyStatus}
                            </button>
                            <p className="cli-login-hint">
                                授权码有效期至 {formatExpiry(authResult.expires_at)}
                            </p>
                            <p className="cli-login-hint">该授权码只能使用一次，请尽快返回 CLI 完成兑换。</p>
                        </div>
                    )}
                </div>
            </div>
        </div>
    )
}
