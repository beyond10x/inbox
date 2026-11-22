import type {ReactNode} from 'react';
import Layout from '@theme/Layout';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import {FeatureGrid, Flow, ProductHero, ProductSection} from '@beyond10x/docs-system/product';
import {landingDescription, landingKpis, landingTitle, type ProductLandingData} from '@beyond10x/docs-system/product-data';
import landingJson from '@site/product.json';
import InboxHero from '@site/src/components/InboxHero';
import PitchSlider from '@site/src/components/PitchSlider';

const landing = landingJson as unknown as ProductLandingData;

const SECTION_IDS: Record<string, string> = {features: 'what', flow: 'how'};

function Section({section}: {section: ProductLandingData['sections'][number]}): ReactNode {
  switch (section.kind) {
    case 'features':
      return <FeatureGrid items={section.items} />;
    case 'flow':
      return <Flow steps={section.steps} />;
    default:
      throw new Error(`the landing page renders sections of kind features and flow, not ${section.kind}`);
  }
}

/** The landing page: the docs-system product hero with the inbox figure beside it, the 3 slides, then the sections of product.json. */
export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  const {product} = landing;
  const first = 1;
  return (
    <Layout title={landingTitle(product, siteConfig.title)} description={landingDescription(product)} wrapperClassName="b10x-product-page">
      <main id="b10x-product-main">
        <div className="b10x-product inbox-landing">
          <ProductHero
            eyebrow={product.eyebrow}
            promise={product.promise}
            lede={product.lede}
            actions={product.actions}
            meta={product.meta}
            aside={<InboxHero />}
            kpis={landingKpis(landing)}
          />
          <PitchSlider />
          {landing.sections.map((section, index) => (
            <ProductSection
              key={`${section.kind}-${index}`}
              id={SECTION_IDS[section.kind]}
              number={first + index}
              eyebrow={section.eyebrow}
              title={section.title}
              lede={section.lede}
              kind={section.kind}>
              <Section section={section} />
            </ProductSection>
          ))}
        </div>
      </main>
    </Layout>
  );
}
