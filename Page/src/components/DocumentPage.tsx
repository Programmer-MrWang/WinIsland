import { useEffect, useMemo, useRef, useState, type ComponentProps } from 'react'
import { BookOpen, ChevronDown, ChevronRight } from 'lucide-react'
import dos from 'highlight.js/lib/languages/dos'
import powershell from 'highlight.js/lib/languages/powershell'
import ReactMarkdown from 'react-markdown'
import { Link } from 'react-router-dom'
import rehypeHighlight from 'rehype-highlight'
import { common } from 'lowlight'
import remarkGfm from 'remark-gfm'
import { copy, DOC_KEYS, localePath, type DocKey, type Locale } from '../content'
import { docs } from '../docs'

type RehypePlugin = NonNullable<ComponentProps<typeof ReactMarkdown>['rehypePlugins']>[number]

const syntaxHighlighting: RehypePlugin = [
  rehypeHighlight,
  {
    aliases: {
      dos: ['bat', 'batch', 'cmd'],
      ini: ['toml'],
      powershell: ['ps1'],
      shell: ['sh'],
    },
    detect: false,
    languages: { ...common, dos, powershell },
    plainText: ['text', 'txt', 'plaintext'],
  },
]

export default function DocumentPage({ locale, page }: { locale: Locale; page: DocKey }) {
  const text = copy[locale]
  const markdown = docs[locale][page]
  const [navigationOpen, setNavigationOpen] = useState(false)
  const navigationRef = useRef<HTMLElement>(null)
  const activePageRef = useRef<HTMLAnchorElement>(null)

  useEffect(() => {
    const navigation = navigationRef.current
    const activePage = activePageRef.current
    if (navigation && activePage && navigation.clientHeight > 0) {
      navigation.scrollTop =
        activePage.offsetTop - (navigation.clientHeight - activePage.offsetHeight) / 2
    }
  }, [locale, page, navigationOpen])

  const markdownComponents = useMemo(
    () => ({
      table: ({ children }: React.ComponentPropsWithoutRef<'table'>) => (
        <div className="markdown-table">
          <table>{children}</table>
        </div>
      ),
      a: ({ href, children, ...props }: React.ComponentPropsWithoutRef<'a'>) => {
        if (!href || href.startsWith('http') || href.startsWith('mailto:')) {
          return (
            <a
              href={href}
              target={href?.startsWith('http') ? '_blank' : undefined}
              rel="noreferrer"
              {...props}
            >
              {children}
            </a>
          )
        }
        const normalized = href.startsWith('/zh/')
          ? href
          : localePath(locale, href.startsWith('/') ? href : `/${href}`)
        return <Link to={normalized}>{children}</Link>
      },
    }),
    [locale],
  )

  return (
    <div className="docs-page page-top section-shell">
      <aside className="docs-sidebar">
        <span>{text.docs.onThisPage}</span>
        <button
          type="button"
          className="docs-nav-toggle"
          aria-expanded={navigationOpen}
          aria-controls="docs-navigation"
          onClick={() => setNavigationOpen((open) => !open)}
        >
          <span>{text.docs.onThisPage}<strong>{text.docs.pages[page]}</strong></span>
          <ChevronDown size={18} />
        </button>
        <nav
          id="docs-navigation"
          ref={navigationRef}
          className={navigationOpen ? 'is-open' : undefined}
          aria-label={text.docs.onThisPage}
        >
          {DOC_KEYS.map((key) => (
            <Link
              key={key}
              ref={page === key ? activePageRef : undefined}
              className={`${page === key ? 'is-active' : ''}${key.startsWith('plugin-dev/') ? ' is-subpage' : ''}${key.startsWith('plugin-dev/api/') ? ' is-api-page' : ''}`.trim()}
              aria-current={page === key ? 'page' : undefined}
              to={localePath(locale, `/${key}`)}
            >
              {text.docs.pages[key]}
            </Link>
          ))}
        </nav>
      </aside>
      <article className="markdown-body">
        <div className="docs-breadcrumb">
          <BookOpen size={16} />
          {text.docs.title}
          <ChevronRight size={14} />
          {text.docs.pages[page]}
        </div>
        <ReactMarkdown
          remarkPlugins={[remarkGfm]}
          rehypePlugins={[syntaxHighlighting]}
          components={markdownComponents}
        >
          {markdown}
        </ReactMarkdown>
      </article>
    </div>
  )
}
