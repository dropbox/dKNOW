#!/usr/bin/env python3
"""
Generate synthetic IDML (InDesign Markup Language) test files for docling_rs.

IDML Structure:
- ZIP archive containing XML files
- designmap.xml: Document structure map
- Stories/*.xml: Text content
- Spreads/*.xml: Page layout
- Resources/Styles.xml: Style definitions
"""

import os
import zipfile
from pathlib import Path

def create_designmap_xml(story_paths, spread_paths, title="Untitled", author="Unknown"):
    """Create designmap.xml content."""
    xml = f'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Document DOMVersion="16.0" Self="d">
'''
    for story_path in story_paths:
        xml += f'  <idPkg:Story src="{story_path}" xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging" />\n'

    for spread_path in spread_paths:
        xml += f'  <idPkg:Spread src="{spread_path}" xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging" />\n'

    xml += f'''  <MetadataPacketPreference>
    <Properties>
      <Title>{title}</Title>
      <Creator>{author}</Creator>
    </Properties>
  </MetadataPacketPreference>
</Document>
'''
    return xml

def create_story_xml(story_id, paragraphs):
    """Create Story XML content with paragraphs."""
    xml = f'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Story Self="{story_id}" AppliedTOCStyle="n" TrackChanges="false" StoryTitle="$ID/" AppliedNamedGrid="n">
  <StoryPreference OpticalMarginAlignment="false" OpticalMarginSize="12" />
'''

    for para in paragraphs:
        style = para.get('style', 'BodyText')
        text = para.get('text', '')

        xml += f'''  <ParagraphStyleRange AppliedParagraphStyle="ParagraphStyle/{style}">
    <CharacterStyleRange AppliedCharacterStyle="CharacterStyle/$ID/[No character style]">
      <Content>{escape_xml(text)}</Content>
    </CharacterStyleRange>
  </ParagraphStyleRange>
'''

    xml += '</Story>\n'
    return xml

def create_spread_xml(spread_id, page_bounds="0 0 612 792", text_frame_story="u123"):
    """Create Spread XML content."""
    xml = f'''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Spread Self="{spread_id}" FlattenerOverride="Default" BindingLocation="1" PageCount="1" ShowMasterItems="true">
  <Page Self="{spread_id}_page1" AppliedMaster="n" GeometricBounds="{page_bounds}" ItemTransform="1 0 0 1 0 0">
    <TextFrame Self="{spread_id}_textframe1" ParentStory="{text_frame_story}" ItemTransform="1 0 0 1 72 72" GeometricBounds="72 72 540 540">
      <Properties>
        <PathGeometry>
          <GeometryPathType PathOpen="false">
            <PathPointArray>
              <PathPointType Anchor="72 72" LeftDirection="72 72" RightDirection="72 72" />
              <PathPointType Anchor="72 540" LeftDirection="72 540" RightDirection="72 540" />
              <PathPointType Anchor="540 540" LeftDirection="540 540" RightDirection="540 540" />
              <PathPointType Anchor="540 72" LeftDirection="540 72" RightDirection="540 72" />
            </PathPointArray>
          </GeometryPathType>
        </PathGeometry>
      </Properties>
    </TextFrame>
  </Page>
</Spread>
'''
    return xml

def create_styles_xml():
    """Create Resources/Styles.xml content."""
    xml = '''<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<idPkg:Styles xmlns:idPkg="http://ns.adobe.com/AdobeInDesign/idml/1.0/packaging">
  <RootParagraphStyleGroup Self="u123">
    <ParagraphStyle Self="ParagraphStyle/Heading1" Name="Heading1" FontStyle="Bold" PointSize="18" />
    <ParagraphStyle Self="ParagraphStyle/Heading2" Name="Heading2" FontStyle="Bold" PointSize="14" />
    <ParagraphStyle Self="ParagraphStyle/BodyText" Name="BodyText" FontStyle="Regular" PointSize="12" />
  </RootParagraphStyleGroup>
  <RootCharacterStyleGroup Self="u456">
    <CharacterStyle Self="CharacterStyle/$ID/[No character style]" Name="$ID/[No character style]" />
  </RootCharacterStyleGroup>
</idPkg:Styles>
'''
    return xml

def escape_xml(text):
    """Escape XML special characters."""
    return (text.replace('&', '&amp;')
                .replace('<', '&lt;')
                .replace('>', '&gt;')
                .replace('"', '&quot;')
                .replace("'", '&apos;'))

def create_idml_file(output_path, title, author, stories):
    """Create an IDML file with stories."""
    with zipfile.ZipFile(output_path, 'w', zipfile.ZIP_DEFLATED) as zf:
        # Create story files
        story_paths = []
        for i, story in enumerate(stories):
            story_id = f"u{1000 + i}"
            story_path = f"Stories/Story_{story_id}.xml"
            story_paths.append(story_path)

            story_xml = create_story_xml(story_id, story['paragraphs'])
            zf.writestr(story_path, story_xml)

        # Create spread files (one per story for simplicity)
        spread_paths = []
        for i, story in enumerate(stories):
            spread_id = f"u{2000 + i}"
            spread_path = f"Spreads/Spread_{spread_id}.xml"
            spread_paths.append(spread_path)

            story_id = f"u{1000 + i}"
            spread_xml = create_spread_xml(spread_id, text_frame_story=story_id)
            zf.writestr(spread_path, spread_xml)

        # Create designmap.xml
        designmap_xml = create_designmap_xml(story_paths, spread_paths, title, author)
        zf.writestr('designmap.xml', designmap_xml)

        # Create Resources/Styles.xml
        styles_xml = create_styles_xml()
        zf.writestr('Resources/Styles.xml', styles_xml)

        # Create mimetype file (IDML standard)
        zf.writestr('mimetype', 'application/vnd.adobe.indesign-idml-package', compress_type=zipfile.ZIP_STORED)

    print(f"Created: {output_path}")

def main():
    """Generate 5 diverse IDML test files."""

    # 1. Simple document (1-page letter)
    create_idml_file(
        'simple_document.idml',
        title='Simple Letter',
        author='John Doe',
        stories=[{
            'paragraphs': [
                {'style': 'Heading1', 'text': 'Business Letter'},
                {'style': 'BodyText', 'text': 'Dear valued customer,'},
                {'style': 'BodyText', 'text': 'Thank you for your recent inquiry. We are pleased to inform you that your order has been processed and will ship within 3-5 business days.'},
                {'style': 'BodyText', 'text': 'If you have any questions, please do not hesitate to contact our customer service team.'},
                {'style': 'BodyText', 'text': 'Sincerely,'},
                {'style': 'BodyText', 'text': 'Customer Service Department'},
            ]
        }]
    )

    # 2. Magazine layout (multi-story article)
    create_idml_file(
        'magazine_layout.idml',
        title='Tech Magazine: AI Revolution',
        author='Jane Smith',
        stories=[
            {
                'paragraphs': [
                    {'style': 'Heading1', 'text': 'The AI Revolution is Here'},
                    {'style': 'Heading2', 'text': 'How Machine Learning is Transforming Industries'},
                    {'style': 'BodyText', 'text': 'Artificial intelligence has moved from science fiction to everyday reality. From smartphones that recognize our faces to cars that drive themselves, AI is reshaping the world around us.'},
                    {'style': 'BodyText', 'text': 'The technology powering this revolution is called machine learning. Unlike traditional software that follows predefined rules, machine learning systems learn from data and improve over time.'},
                ]
            },
            {
                'paragraphs': [
                    {'style': 'Heading2', 'text': 'Key Applications'},
                    {'style': 'BodyText', 'text': 'Healthcare: AI systems can now detect diseases from medical images with accuracy matching or exceeding human radiologists.'},
                    {'style': 'BodyText', 'text': 'Finance: Machine learning algorithms detect fraud, manage risk, and even trade stocks automatically.'},
                    {'style': 'BodyText', 'text': 'Transportation: Self-driving cars use AI to navigate roads, avoid obstacles, and make split-second decisions.'},
                ]
            }
        ]
    )

    # 3. Brochure (tri-fold marketing material)
    create_idml_file(
        'brochure.idml',
        title='Acme Products Brochure',
        author='Marketing Department',
        stories=[{
            'paragraphs': [
                {'style': 'Heading1', 'text': 'Acme Products'},
                {'style': 'Heading2', 'text': 'Quality Solutions Since 1985'},
                {'style': 'BodyText', 'text': 'Discover our wide range of innovative products designed to make your life easier. From household essentials to professional tools, Acme has what you need.'},
                {'style': 'Heading2', 'text': 'Our Services'},
                {'style': 'BodyText', 'text': '- 24/7 Customer Support'},
                {'style': 'BodyText', 'text': '- Free Shipping on Orders Over $50'},
                {'style': 'BodyText', 'text': '- 30-Day Money-Back Guarantee'},
                {'style': 'BodyText', 'text': '- Expert Product Consultation'},
                {'style': 'Heading2', 'text': 'Contact Us'},
                {'style': 'BodyText', 'text': 'Phone: 1-800-ACME-PROD'},
                {'style': 'BodyText', 'text': 'Email: info@acmeproducts.com'},
                {'style': 'BodyText', 'text': 'Web: www.acmeproducts.com'},
            ]
        }]
    )

    # 4. Book chapter (long-form content)
    create_idml_file(
        'book_chapter.idml',
        title='History of Computing - Chapter 1',
        author='Dr. Robert Chen',
        stories=[{
            'paragraphs': [
                {'style': 'Heading1', 'text': 'Chapter 1: The Birth of Computing'},
                {'style': 'BodyText', 'text': 'The story of modern computing begins not with silicon chips or transistors, but with mechanical calculators and the dreams of mathematicians who imagined machines that could think.'},
                {'style': 'Heading2', 'text': 'Early Mechanical Computers'},
                {'style': 'BodyText', 'text': 'In 1822, Charles Babbage proposed the Difference Engine, a mechanical calculator designed to tabulate polynomial functions. Though never completed in his lifetime, Babbage\'s design laid the groundwork for all computers to come.'},
                {'style': 'BodyText', 'text': 'Babbage\'s more ambitious design, the Analytical Engine, was truly revolutionary. It featured programmable instructions stored on punched cards, an arithmetic logic unit, control flow through conditional branching, and memory - all the essential components of a modern computer.'},
                {'style': 'Heading2', 'text': 'The Turing Machine'},
                {'style': 'BodyText', 'text': 'In 1936, Alan Turing published his groundbreaking paper "On Computable Numbers," which introduced the concept of the Turing Machine. This theoretical device could perform any calculation that could be described by an algorithm.'},
                {'style': 'BodyText', 'text': 'Turing\'s work provided the mathematical foundation for computer science and proved fundamental limits on what computers can and cannot compute. His insights during World War II, when he helped break the German Enigma code, demonstrated the practical power of computational thinking.'},
                {'style': 'Heading2', 'text': 'The Electronic Era'},
                {'style': 'BodyText', 'text': 'The first fully electronic computer, ENIAC (Electronic Numerical Integrator and Computer), was completed in 1945. It weighed 30 tons and filled an entire room, but it could perform calculations thousands of times faster than any mechanical device.'},
                {'style': 'BodyText', 'text': 'ENIAC marked the beginning of a new era. Within a decade, transistors would replace vacuum tubes. Within two decades, integrated circuits would miniaturize entire computers onto single chips. The digital revolution had begun.'},
            ]
        }]
    )

    # 5. Technical manual (code samples and lists)
    create_idml_file(
        'technical_manual.idml',
        title='API Documentation: User Authentication',
        author='Engineering Team',
        stories=[{
            'paragraphs': [
                {'style': 'Heading1', 'text': 'User Authentication API'},
                {'style': 'Heading2', 'text': 'Overview'},
                {'style': 'BodyText', 'text': 'The User Authentication API provides secure login and session management for your application. All endpoints use HTTPS and require API key authentication.'},
                {'style': 'Heading2', 'text': 'Base URL'},
                {'style': 'BodyText', 'text': 'https://api.example.com/v1/auth'},
                {'style': 'Heading2', 'text': 'Authentication Flow'},
                {'style': 'BodyText', 'text': '1. Client sends credentials to /login endpoint'},
                {'style': 'BodyText', 'text': '2. Server validates credentials and returns JWT token'},
                {'style': 'BodyText', 'text': '3. Client includes token in Authorization header for subsequent requests'},
                {'style': 'BodyText', 'text': '4. Token expires after 24 hours; refresh using /refresh endpoint'},
                {'style': 'Heading2', 'text': 'Example: Login Request'},
                {'style': 'BodyText', 'text': 'POST /login HTTP/1.1'},
                {'style': 'BodyText', 'text': 'Content-Type: application/json'},
                {'style': 'BodyText', 'text': ''},
                {'style': 'BodyText', 'text': '{"username": "user@example.com", "password": "secret123"}'},
                {'style': 'Heading2', 'text': 'Response'},
                {'style': 'BodyText', 'text': '{"token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...", "expires_in": 86400}'},
                {'style': 'Heading2', 'text': 'Error Codes'},
                {'style': 'BodyText', 'text': '400 Bad Request - Invalid input format'},
                {'style': 'BodyText', 'text': '401 Unauthorized - Invalid credentials'},
                {'style': 'BodyText', 'text': '429 Too Many Requests - Rate limit exceeded'},
                {'style': 'BodyText', 'text': '500 Internal Server Error - Server error'},
            ]
        }]
    )

    print("\nAll 5 IDML test files created successfully!")
    print("\nTest files:")
    print("1. simple_document.idml - Simple business letter")
    print("2. magazine_layout.idml - Multi-story magazine article")
    print("3. brochure.idml - Tri-fold marketing brochure")
    print("4. book_chapter.idml - Long-form book chapter")
    print("5. technical_manual.idml - API documentation with code samples")

if __name__ == '__main__':
    main()
