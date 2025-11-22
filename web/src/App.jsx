import { useState, useEffect, useRef } from 'react'
import axios from 'axios'

function App() {
  const [user, setUser] = useState(null)
  const [error, setError] = useState(null)
  const [registrationStatus, setRegistrationStatus] = useState(null)
  const [loading, setLoading] = useState(false)
  const [activeTab, setActiveTab] = useState('pending')
  const [pendingList, setPendingList] = useState([])
  const [archivedList, setArchivedList] = useState([])
  const [dataLoading, setDataLoading] = useState(false)
  const telegramLoginRef = useRef(null)

  // 检查用户注册状态的函数
  const checkUserRegistration = async (telegramId) => {
    setLoading(true)
    try {
      const response = await axios.get(`/api/check_user/${telegramId}`)
      setRegistrationStatus(response.data)
      
      // 如果用户已注册，获取数据
      if (response.data.registered) {
        fetchData()
      }
    } catch (error) {
      console.error('Failed to check user registration:', error)
      setError('检查用户状态失败')
    } finally {
      setLoading(false)
    }
  }

  // 获取所有数据的函数
  const fetchData = async () => {
    setDataLoading(true)
    try {
      const [pendingResponse, archivedResponse] = await Promise.all([
        axios.get('/api/pending'),
        axios.get('/api/archived')
      ])
      setPendingList(pendingResponse.data)
      setArchivedList(archivedResponse.data)
    } catch (error) {
      console.error('Failed to fetch data:', error)
      setError('获取数据失败')
    } finally {
      setDataLoading(false)
    }
  }

  useEffect(() => {
    // 检查是否已有存储的用户信息
    const storedUser = sessionStorage.getItem('telegramUser')
    if (storedUser) {
      try {
        const userData = JSON.parse(storedUser)
        setUser(userData)
        // 如果已经有用户信息，检查注册状态
        checkUserRegistration(userData.id)
      } catch (e) {
        console.error('Failed to parse stored user:', e)
        sessionStorage.removeItem('telegramUser')
      }
    }

    // 检查 URL 参数中是否包含 Telegram 登录回调数据
    const urlParams = new URLSearchParams(window.location.search)
    const id = urlParams.get('id')
    const first_name = urlParams.get('first_name')
    const last_name = urlParams.get('last_name')
    const username = urlParams.get('username')
    const photo_url = urlParams.get('photo_url')
    const auth_date = urlParams.get('auth_date')
    const hash = urlParams.get('hash')

    if (id && auth_date && hash) {
      const telegramData = {
        id,
        first_name,
        last_name,
        username,
        photo_url,
        auth_date,
        hash
      }
      
      // 存储到 sessionStorage
      sessionStorage.setItem('telegramUser', JSON.stringify(telegramData))
      setUser(telegramData)
      
      // 清理 URL 参数
      window.history.replaceState({}, document.title, window.location.pathname)
      
      // 检查用户是否已注册
      checkUserRegistration(telegramData.id)
    }
  }, [])

  useEffect(() => {
    // 定义全局回调函数
    window.onTelegramAuth = function(user) {
      console.log('Telegram auth success:', user)
      
      // 存储到 sessionStorage
      sessionStorage.setItem('telegramUser', JSON.stringify(user))
      setUser(user)
      
      // 重定向到配置的 URL
      const redirectUrl = import.meta.env.VITE_REDIRECT_URL
      if (redirectUrl && redirectUrl !== window.location.href) {
        setTimeout(() => {
          window.location.href = redirectUrl
        }, 1000)
      }
    }

    // 清理函数
    return () => {
      delete window.onTelegramAuth
    }
  }, [])

  useEffect(() => {
    // 创建 Telegram Widget
    if (!user && telegramLoginRef.current) {
      let botUsername = import.meta.env.VITE_BOT_USERNAME
      
      // 如果是 bot ID（纯数字），需要提示用户配置 bot username
      if (!botUsername || /^\d+$/.test(botUsername)) {
        setError('请在 .env 文件中配置正确的 VITE_BOT_USERNAME（bot 用户名，不是 ID）')
        return
      }

      // 移除 @ 前缀（如果有）
      botUsername = botUsername.replace('@', '')

      // 直接使用当前页面作为回调 URL
      const authUrl = window.location.href

      // 清空容器
      telegramLoginRef.current.innerHTML = ''
      
      // 创建登录脚本
      const script = document.createElement('script')
      script.async = true
      script.src = 'https://telegram.org/js/telegram-widget.js?22'
      script.setAttribute('data-telegram-login', botUsername)
      script.setAttribute('data-size', 'large')
      script.setAttribute('data-auth-url', authUrl)
      script.setAttribute('data-request-access', 'write')
      
      telegramLoginRef.current.appendChild(script)
    }
  }, [user])

  if (user) {
    if (loading) {
      return (
        <div className="container">
          <div className="card">
            <h1 className="title">检查用户状态中...</h1>
            <div style={{ textAlign: 'center', padding: '20px' }}>
              <div className="spinner"></div>
            </div>
          </div>
        </div>
      )
    }

    if (registrationStatus) {
      if (!registrationStatus.registered) {
        return (
          <div className="container">
            <div className="card">
              <h1 className="title" style={{ fontSize: '48px', color: '#dc3545', fontWeight: 'bold' }}>
                ACCESS DENIED
              </h1>
              <div style={{ textAlign: 'center', marginBottom: '24px' }}>
                {user.photo_url && (
                  <img 
                    src={user.photo_url} 
                    alt="Avatar" 
                    style={{ 
                      width: '80px', 
                      height: '80px', 
                      borderRadius: '50%', 
                      marginBottom: '16px' 
                    }}
                  />
                )}
                <h2 style={{ marginBottom: '8px' }}>
                  {user.first_name} {user.last_name}
                </h2>
                {user.username && (
                  <p style={{ color: 'var(--tg-theme-hint-color, #6c757d)' }}>
                    @{user.username}
                  </p>
                )}
              </div>
              <p style={{ textAlign: 'center', marginBottom: '24px', color: '#dc3545' }}>
                用户未注册，无法访问
              </p>
              <button 
                onClick={() => {
                  sessionStorage.removeItem('telegramUser')
                  setUser(null)
                  setRegistrationStatus(null)
                }}
                className="login-button"
              >
                重新登录
              </button>
            </div>
          </div>
        )
      } else {
        const currentData = activeTab === 'pending' ? pendingList : 
                           activeTab === 'archived' ? archivedList : []

        return (
          <div className="app-container">
            {/* 顶部用户信息栏 */}
            <header className="user-header">
              <div className="user-info">
                {user.photo_url && (
                  <img 
                    src={user.photo_url} 
                    alt="Avatar" 
                    className="user-avatar"
                  />
                )}
                <div className="user-details">
                  <span className="user-name">
                    {user.username ? `@${user.username}` : `${user.first_name} ${user.last_name}`}
                  </span>
                  <span className="emby-username">
                    Emby: {registrationStatus.database_username}
                  </span>
                </div>
              </div>
              <button 
                onClick={() => {
                  sessionStorage.removeItem('telegramUser')
                  setUser(null)
                  setRegistrationStatus(null)
                  setPendingList([])
                  setArchivedList([])
                }}
                className="logout-button"
              >
                登出
              </button>
            </header>

            {/* 主要内容区域 */}
            <main className="main-content">
              {/* 选项卡导航 */}
              <div className="tabs">
                <button 
                  className={`tab ${activeTab === 'pending' ? 'active' : ''}`}
                  onClick={() => setActiveTab('pending')}
                >
                  未入库 ({pendingList.length})
                </button>
                <button 
                  className={`tab ${activeTab === 'archived' ? 'active' : ''}`}
                  onClick={() => setActiveTab('archived')}
                >
                  已入库 ({archivedList.length})
                </button>
                <button 
                  className={`tab ${activeTab === 'subscriptions' ? 'active' : ''}`}
                  onClick={() => setActiveTab('subscriptions')}
                  disabled
                >
                  我的订阅 (即将推出)
                </button>
              </div>

              {/* 内容区域 */}
              <div className="content">
                {dataLoading && (
                  <div className="loading-container">
                    <div className="spinner"></div>
                    <p>加载数据中...</p>
                  </div>
                )}
                
                {!dataLoading && currentData.length === 0 && (
                  <div className="empty-state">
                    <p>
                      {activeTab === 'pending' ? '暂无未入库内容' : 
                       activeTab === 'archived' ? '暂无已入库内容' : '暂无订阅'}
                    </p>
                  </div>
                )}
                
                {!dataLoading && currentData.length > 0 && (
                  <div className="media-grid">
                    {currentData.map(item => (
                      <div key={item.id} className="media-item">
                        {item.poster && (
                          <img 
                            src={item.poster} 
                            alt={item.title || `${item.source} ${item.media_id}`}
                            className="media-poster"
                            onError={(e) => {
                              e.target.style.display = 'none'
                            }}
                          />
                        )}
                        <h3 className="media-title">
                          {item.title || `${item.source} ${item.media_id}`}
                        </h3>
                        <div className="media-meta">
                          <span className="media-source">{item.source}</span>
                          <span className="media-date">
                            {new Date(item.created_at).toLocaleDateString()}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </main>
          </div>
        )
      }
    }
  }

  return (
    <div className="container">
      <div className="card">
        <h1 className="title">Nyamedia Bot</h1>
        <p className="subtitle">使用 Telegram 账户登录</p>
        
        {error && (
          <div style={{ 
            color: '#dc3545', 
            textAlign: 'center', 
            marginBottom: '16px',
            padding: '12px',
            backgroundColor: '#f8d7da',
            border: '1px solid #f5c6cb',
            borderRadius: '4px'
          }}>
            {error}
          </div>
        )}

        <div ref={telegramLoginRef} style={{ textAlign: 'center' }}>
          {/* Telegram Widget 将在这里动态插入 */}
        </div>
      </div>
    </div>
  )
}

export default App